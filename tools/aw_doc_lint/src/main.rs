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
//!
//! A separate front-door check always fails the process, including in `--mode warn`:
//! `docs/current/ENGINE_STATUS.md` must name every required replay case from
//! `determinism-replay.yml` and must keep the frame-budget phrases. Phrases
//! are matched on the visible text of a pulldown-cmark render (tables,
//! strikethrough, task lists, and footnotes; smart punctuation stays off).
//! `3.117 ms` and
//! `3.688 ms` have to be their own tokens on a sentence with whole-word
//! `median` or `measured`, or whole-word `sample` with whole-word `is` or
//! `was`. `11 ms` is its own token. `13.688 ms`, `3.688 msec`, and `111 ms`
//! do not match. Whole-word `not`, `no`, `none`, `nobody`, `nothing`,
//! `never`, `cannot`, `neither`, or `nor`, or an `n't` contraction, keeps that
//! sentence off the set. The same negation keeps `11 ms`, `Local LLMs propose`,
//! `deterministic simulation validates`, `CC0`, `GTX 1660 Ti`, and `Aravath`
//! off the set. Link labels join the surrounding letters. An autolink keeps
//! its URL. A link reference definition is omitted. The apostrophe may be
//! ASCII, U+2018, U+2019, U+02BC, or U+FF07, including a character reference
//! that decodes to one of those. `Code` stays as pulldown-cmark emitted it.
//! `Text` decodes a numeric character reference with any run of leading
//! zeros, and leaves a named reference as that parser emitted it, so
//! `&amp;not;`, `\&not;`, and `&not ` keep the word `not`, and
//! `3.117&amp;nbsp;ms` does not become U+00A0. Raw HTML and alt text also
//! accept an HTML5 legacy name with no semicolon. In alt text that name is
//! not consumed when the next byte is `=` or an ASCII alphanumeric. A `/` or
//! any ASCII whitespace before `alt` starts that attribute. The fold
//! maps U+FF01–U+FF5E into ASCII and U+3000 into a space, and drops
//! non-whitespace controls, the Cf and Mn characters
//! `unicode_categories` 0.1.1 classifies, the Unicode 15 gaps in `unicode_gaps`,
//! and U+115F, U+1160, U+3164, and U+FFA0. U+1734 stays: Unicode 15 classifies
//! it as Mc, and the crate calls it Mn. An HTML comment is hidden. An unclosed
//! `<!--` hides through the end of that block, including a blank line.
//! `<!-->`, `<!--->`, and `--!>` close the comment. A comment inside a code span or a
//! fence stays. A `script` or `style` body stays. A `br` tag inserts a break.
//! A referenced footnote definition is scanned. An unreferenced one is
//! omitted. A defined footnote reference writes its digits as superscripts,
//! or U+00B9 when the label has no digit. Naming the capture is not a
//! measurement. A sentence that only names the check does not count.
//! `README.md` must list open pulls as in progress and must not contain a
//! frame figure: a number followed by `ms`, `msec`, `millisecond`, or
//! `milliseconds`, including the fullwidth block U+FF01–U+FF5E. The same
//! render supplies that scan. The in-progress section is the rendered h2
//! whose folded text is exactly `in progress`, setext or ATX. It ends at the
//! next rendered h2 after that heading is finished, unless the heading's
//! folded text, with every run of Unicode spaces mapped to one ASCII space
//! and trimmed, is `in progress` or empty. Its issue numbers are read from
//! that section after the same fold. The
//! in-progress words and the retired title are read from that render after
//! the fold. Quoted and unquoted `alt` text stays. A link or image title, and
//! an HTML `title` attribute, are scanned on their own line at the end of the
//! enclosing block. A
//! non-comment `<` that never reaches `>` is text. A backslash immediately
//! before a newline is a hard break. In raw HTML and in alt text, a numeric
//! character reference follows the HTML tokenizer: the semicolon is
//! optional, leading zeros do not count, 0 is U+FFFD, and 0x80–0x9F use the
//! Windows-1252 row. The five undefined slots (0x81, 0x8D, 0x8F, 0x90, 0x9D)
//! are U+FFFD. A markdown numeric reference in 0x80–0x9F is U+FFFD, not that
//! row. C0 controls other than tab, line feed, form feed, and carriage
//! return, and DEL, are U+FFFD in that text and in raw HTML, for a decoded
//! reference and for a raw scalar. The fold keeps U+FFFD. Form feed stays
//! whitespace. Named references are the HTML5 table in `html_entities`, and
//! the 106 legacy names in `html_legacy` need no semicolon. Markdown text
//! keeps that semicolon rule for names.
//! Invisible characters are the
//! crate's Cf and Mn, except U+1734, the gaps in `unicode_gaps`, those four
//! Default_Ignorable letters, and non-whitespace controls, in
//! `is_invisible_frame_char`. Player-facing docs must not use the retired game
//! title. A failed read of a scanned file, including
//! `docs/current/PROJECT_BRIEF.md`, fails the process.
//!
//! Merge/rebase conflict markers (`<<<<<<<`, `|||||||`, `=======` between
//! open and close, `>>>>>>>`) are always hard errors outside fenced code
//! blocks; see `conflict_markers.rs` and `tools/aw_doc_lint/README.md`.

mod conflict_markers;
mod html_entities;
mod html_legacy;
mod rendered;
mod unicode_gaps;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use unicode_categories::UnicodeCategories;

const SPEC_REL: &str = "docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md";
const ALLOWLIST_REL: &str = "docs/campaigns/doc-truth/CLOSED_VOCABULARY_ALLOWLIST.md";
const REPLAY_WORKFLOW_REL: &str = ".github/workflows/determinism-replay.yml";
const README_REL: &str = "README.md";
const STATUS_REL: &str = "docs/current/ENGINE_STATUS.md";

const IN_PROGRESS_ISSUES: &[&str] = &["#334", "#366", "#383"];

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

fn ascii_word(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
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

// ---- front door (always enforced) ------------------------------------------

fn is_player_facing(path: &str) -> bool {
    let p = path.replace('\\', "/");
    p == "README.md"
        || p == "CONTRIBUTING.md"
        || p == "docs/README.md"
        || p == "docs/QUICKSTART.md"
        || p == "docs/current/PROJECT_BRIEF.md"
        || (p.starts_with("docs/guides/") && p.ends_with(".md"))
}

/// Quoted names inside each `required = [` / `required = {` assignment.
fn required_case_lists(workflow: &str) -> Vec<Vec<String>> {
    let mut lists = Vec::new();
    for (i, _) in workflow.match_indices("required") {
        let after = &workflow[i..];
        let Some(eq) = after.find('=') else { continue };
        if eq > 24 {
            continue;
        }
        let rest = after[eq + 1..].trim_start();
        let close = if rest.starts_with('[') {
            ']'
        } else if rest.starts_with('{') {
            '}'
        } else {
            continue;
        };
        let Some(end) = rest.find(close) else {
            continue;
        };
        let names: Vec<String> = rest[1..end]
            .split(',')
            .filter_map(|part| {
                let name = part.trim().trim_matches('"').trim_matches('\'').trim();
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            })
            .collect();
        if !names.is_empty() {
            lists.push(names);
        }
    }
    lists
}

fn contains_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    haystack.match_indices(word).any(|(idx, _)| {
        let end = idx + word.len();
        let left_ok = idx == 0 || !is_word_byte(bytes[idx - 1]);
        let right_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        left_ok && right_ok
    })
}

fn in_progress_body(readme: &str) -> Option<String> {
    rendered::in_progress_section(readme)
}

/// `#123` tokens. A digit run that is not a whole issue number is skipped.
fn issue_numbers(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'#' {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            let left_ok = index == 0 || !is_word_byte(bytes[index - 1]);
            let right_ok = end == bytes.len() || !is_word_byte(bytes[end]);
            if end > start && left_ok && right_ok {
                let digits = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
                if !digits.is_empty() {
                    found.push(format!("#{digits}"));
                }
            }
            index = end;
        } else {
            index += 1;
        }
    }
    found
}

fn has_bare_word(text: &str, word: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let needle = word.to_ascii_lowercase();
    contains_word(&lower, &needle)
}

/// Retired game title in player-facing prose. Identifier suffixes (`veilweaver_demo`)
/// and the retained archive directory (`projects/veilweaver`) are not title uses.
fn retired_title_hits(path: &str, text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let needle = "veilweaver";
    let bytes = lower.as_bytes();
    let mut hits = Vec::new();
    let mut from = 0;
    while let Some(rel) = lower[from..].find(needle) {
        let idx = from + rel;
        let end = idx + needle.len();
        let prev_ok = idx == 0 || !is_word_byte(bytes[idx - 1]);
        let ident = bytes.get(end).copied() == Some(b'_');
        // Byte window, not a `str` slice: `idx - 9` can sit inside a multibyte char.
        let path_dir = bytes[..idx].ends_with(b"projects/");
        if prev_ok && !ident && !path_dir {
            let line = text[..idx].bytes().filter(|&b| b == b'\n').count() + 1;
            hits.push(format!(
                "{path}:{line}: player-facing text uses the retired game title"
            ));
        }
        from = end;
    }
    hits
}

fn front_door_errors(
    readme: &str,
    status: &str,
    workflow: &str,
    player_facing: &[(String, String)],
) -> Vec<String> {
    let mut errors = Vec::new();
    let lists = required_case_lists(workflow);
    if lists.len() < 2 {
        errors.push(format!(
            "{REPLAY_WORKFLOW_REL}: expected two required-case lists, found {}",
            lists.len()
        ));
    } else if lists.iter().any(|list| list != &lists[0]) {
        errors.push(format!(
            "{REPLAY_WORKFLOW_REL}: required-case lists disagree"
        ));
    }
    let visible = status_front_door_text(status);
    if let Some(cases) = lists.first() {
        for case in cases {
            if !contains_word(&visible, case) {
                errors.push(format!(
                    "{STATUS_REL}: missing required replay case `{case}`"
                ));
            }
        }
    }
    for phrase in [
        "3.688 ms",
        "11 ms",
        "GTX 1660 Ti",
        "CC0",
        "Local LLMs propose",
        "deterministic simulation validates",
        "Aravath",
    ] {
        let present = match phrase {
            "3.688 ms" => measured_sentence_has_token(&visible, phrase),
            "11 ms" => non_negated_sentence_has_token(&visible, phrase),
            _ => non_negated_sentence_contains(&visible, phrase),
        };
        if !present {
            errors.push(format!(
                "{STATUS_REL}: missing front-door phrase `{phrase}`"
            ));
        }
    }
    if !status_has_measured_sample(status, "3.117 ms") {
        errors.push(format!(
            "{STATUS_REL}: missing front-door phrase `3.117 ms` on a measurement sentence"
        ));
    }
    match in_progress_body(readme) {
        None => errors.push(format!("{README_REL}: missing a '## In progress' section")),
        Some(body) => {
            let visible_body = fold_visible_text(&body);
            let found = issue_numbers(&visible_body);
            for issue in IN_PROGRESS_ISSUES {
                if !found.iter().any(|token| token == issue) {
                    errors.push(format!("{README_REL}: in-progress section missing {issue}"));
                }
            }
            for token in &found {
                if !IN_PROGRESS_ISSUES.contains(&token.as_str()) {
                    errors.push(format!(
                        "{README_REL}: in-progress section lists {token}, which is not in the in-progress list"
                    ));
                }
            }
            for word in ["shipped", "landed", "merged"] {
                if has_bare_word(&visible_body, word) {
                    errors.push(format!(
                        "{README_REL}: in-progress section marks work {word}"
                    ));
                }
            }
        }
    }
    errors.extend(readme_frame_figure_errors(readme));
    for (path, text) in player_facing {
        errors.extend(retired_title_hits(
            path,
            &fold_visible_text(&rendered::render_visible(text)),
        ));
    }
    errors
}

/// The sample must be its own token on a measurement sentence. `13.117 ms` is
/// not `3.117 ms`. The sentence needs whole-word `median` or `measured`, or
/// whole-word `sample` together with whole-word `is` or `was`. Naming a
/// `capture` does not count. Whole-word `not`, `no`, `none`, `nobody`,
/// `nothing`, `never`, `cannot`, `neither`, or `nor`, or an `n't` contraction,
/// does not count. The sentence is read from the rendered visible text.
/// Other sentences on the same line still can.
fn status_has_measured_sample(status: &str, phrase: &str) -> bool {
    measured_sentence_has_token(&status_front_door_text(status), phrase)
}

fn measured_sentence_has_token(visible: &str, phrase: &str) -> bool {
    visible.lines().any(|line| {
        line.split(". ").any(|sentence| {
            contains_phrase_token(sentence, phrase) && sentence_is_measurement(sentence)
        })
    })
}

fn status_front_door_text(status: &str) -> String {
    fold_visible_text(&rendered::render_visible(status))
}

/// Fold fullwidth forms, drop invisible characters, and fold apostrophes to
/// ASCII `'`. Numeric references in `Text` are already decoded, so this pass
/// does not read character references.
fn fold_visible_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        push_visible_char(&mut out, ch);
    }
    out
}

fn push_visible_char(out: &mut String, ch: char) {
    let ch = fold_fullwidth(ch);
    if is_invisible_frame_char(ch) {
        return;
    }
    out.push(if is_apostrophe(ch) { '\'' } else { ch });
}

fn non_negated_sentence_contains(visible: &str, phrase: &str) -> bool {
    visible.lines().any(|line| {
        line.split(". ").any(|sentence| {
            sentence.contains(phrase) && !negated_measurement(&sentence.to_ascii_lowercase())
        })
    })
}

fn non_negated_sentence_has_token(visible: &str, phrase: &str) -> bool {
    visible.lines().any(|line| {
        line.split(". ").any(|sentence| {
            contains_phrase_token(sentence, phrase)
                && !negated_measurement(&sentence.to_ascii_lowercase())
        })
    })
}

fn is_apostrophe(ch: char) -> bool {
    matches!(ch, '\'' | '\u{2018}' | '\u{2019}' | '\u{02BC}')
}

fn contains_phrase_token(haystack: &str, phrase: &str) -> bool {
    let lower = haystack.to_ascii_lowercase();
    let phrase = phrase.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let needle = phrase.as_bytes();
    if needle.is_empty() {
        return false;
    }
    let mut from = 0;
    while let Some(window) = lower.get(from..) {
        let Some(rel) = window.find(phrase.as_str()) else {
            break;
        };
        let idx = from + rel;
        let end = idx + needle.len();
        let prev_ok = idx == 0 || !is_word_byte(bytes[idx - 1]);
        let next_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        if prev_ok && next_ok {
            return true;
        }
        // One byte later. When the match starts on a multibyte character, that
        // byte is inside the character, so resume on the next boundary. An
        // ASCII match is already one byte wide, so this stays `idx + 1`.
        from = idx + 1;
        if !lower.is_char_boundary(from) {
            let Some(ch) = lower[idx..].chars().next() else {
                break;
            };
            from = idx + ch.len_utf8();
        }
    }
    false
}

fn sentence_is_measurement(sentence: &str) -> bool {
    let lower = sentence.to_ascii_lowercase();
    if lower.contains("aw_doc_lint")
        || lower.contains("front door")
        || lower.contains("front-door")
        || lower.contains("looks for")
    {
        return false;
    }
    if negated_measurement(&lower) {
        return false;
    }
    contains_phrase_token(&lower, "median")
        || contains_phrase_token(&lower, "measured")
        || (contains_phrase_token(&lower, "sample")
            && (contains_phrase_token(&lower, "is") || contains_phrase_token(&lower, "was")))
}

fn negated_measurement(lower: &str) -> bool {
    contains_phrase_token(lower, "not")
        || contains_phrase_token(lower, "no")
        || contains_phrase_token(lower, "none")
        || contains_phrase_token(lower, "nobody")
        || contains_phrase_token(lower, "nothing")
        || contains_phrase_token(lower, "never")
        || contains_phrase_token(lower, "cannot")
        || contains_phrase_token(lower, "neither")
        || contains_phrase_token(lower, "nor")
        || lower.contains("n't")
}

/// A frame figure is a number followed by `ms`, `msec`, `millisecond`, or
/// `milliseconds` in the rendered visible text.
fn readme_frame_figure_errors(readme: &str) -> Vec<String> {
    let pieces = frame_pieces(readme);
    let mut errors = Vec::new();
    let mut i = 0;
    while i < pieces.len() {
        if !pieces[i].0.is_ascii_digit() {
            i += 1;
            continue;
        }
        if i > 0 && ascii_word(pieces[i - 1].0) {
            i += 1;
            continue;
        }
        let line = pieces[i].1;
        i += 1;
        while i < pieces.len() && (pieces[i].0.is_ascii_digit() || pieces[i].0 == '.') {
            i += 1;
        }
        while i < pieces.len() && pieces[i].0 == ' ' {
            i += 1;
        }
        let unit = frame_unit(&pieces[i..]);
        if unit == 0 {
            continue;
        }
        let after = i + unit;
        if after < pieces.len() && pieces[after].0.is_ascii_alphanumeric() {
            continue;
        }
        errors.push(format!(
            "{README_REL}:{line}: frame figure is not allowed on the landing page"
        ));
        i = after;
    }
    errors
}

fn frame_unit(pieces: &[(char, usize)]) -> usize {
    const MILLISECONDS: &[char] = &['m', 'i', 'l', 'l', 'i', 's', 'e', 'c', 'o', 'n', 'd', 's'];
    const MILLISECOND: &[char] = &['m', 'i', 'l', 'l', 'i', 's', 'e', 'c', 'o', 'n', 'd'];
    const MSEC: &[char] = &['m', 's', 'e', 'c'];
    const MS: &[char] = &['m', 's'];
    if pieces_start(pieces, MILLISECONDS) {
        MILLISECONDS.len()
    } else if pieces_start(pieces, MILLISECOND) {
        MILLISECOND.len()
    } else if pieces_start(pieces, MSEC) {
        MSEC.len()
    } else if pieces_start(pieces, MS) {
        MS.len()
    } else {
        0
    }
}

fn pieces_start(pieces: &[(char, usize)], prefix: &[char]) -> bool {
    pieces.len() >= prefix.len() && pieces.iter().zip(prefix).all(|(piece, ch)| piece.0 == *ch)
}

fn frame_pieces(input: &str) -> Vec<(char, usize)> {
    let visible = fold_visible_text(&rendered::render_visible(input));
    let mut out = Vec::new();
    let mut line = 1usize;
    for ch in visible.chars() {
        if ch == '\n' {
            line += 1;
            push_frame_char(&mut out, ' ', line);
        } else {
            push_frame_char(&mut out, ch, line);
        }
    }
    out
}

fn push_frame_char(out: &mut Vec<(char, usize)>, ch: char, line: usize) {
    let ch = fold_fullwidth(ch);
    if is_invisible_frame_char(ch) {
        return;
    }
    let ch = if ch.is_whitespace() {
        ' '
    } else {
        ch.to_ascii_lowercase()
    };
    out.push((ch, line));
}

/// Map the fullwidth block U+FF01–U+FF5E onto ASCII, and U+3000 onto a space.
/// This is that block only, not every NFKC compatibility character.
fn fold_fullwidth(ch: char) -> char {
    match ch {
        '\u{FF01}'..='\u{FF5E}' => char::from_u32(u32::from(ch) - 0xFEE0).unwrap_or(ch),
        '\u{3000}' => ' ',
        other => other,
    }
}

/// Characters GitHub does not draw between a number and its unit.
///
/// Whitespace, including U+200A and U+00A0, stays and becomes a normal space.
/// `unicode_categories` 0.1.1 classifies Cf, Mn, and Cc, but its tables are not
/// Unicode 15: they miss the 20 Cf and 419 Mn characters in `unicode_gaps`
/// (U+08E2 and U+180F among them). U+061C, U+180E, and U+E0001 are in the
/// crate. U+1734 is in the crate as Mn; Unicode 15 classifies it as Mc, so it
/// stays. U+115F, U+1160, U+3164, and U+FFA0 are Default_Ignorable letters, not
/// Cf or Mn, and are dropped here too. Other non-whitespace controls are
/// dropped. This is not every Cf or Mn in a newer Unicode, and it is not U+33B3.
fn is_invisible_frame_char(ch: char) -> bool {
    if ch.is_whitespace() || ch == '\u{1734}' {
        return false;
    }
    if matches!(ch, '\u{115F}' | '\u{1160}' | '\u{3164}' | '\u{FFA0}') {
        return true;
    }
    ch.is_other_format()
        || ch.is_mark_nonspacing()
        || ch.is_other_control()
        || unicode_gaps::is_gap(ch)
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

fn read_repo_file(root: &Path, rel: &str) -> Result<String> {
    std::fs::read_to_string(root.join(rel)).with_context(|| format!("reading {rel}"))
}

fn load_player_facing(root: &Path, tracked: &[String]) -> Result<Vec<(String, String)>> {
    let mut player_facing = Vec::new();
    for f in tracked {
        if !is_player_facing(f) {
            continue;
        }
        player_facing.push((f.clone(), read_repo_file(root, f)?));
    }
    Ok(player_facing)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    lint_repo(&resolve_root(&cli)?, cli.mode)
}

fn lint_repo(root: &Path, mode: Mode) -> Result<()> {
    let spec = read_repo_file(root, SPEC_REL).context("reading the closed-vocabulary spec")?;
    let allowlist_doc =
        read_repo_file(root, ALLOWLIST_REL).context("reading the closed-vocabulary allowlist")?;
    let lits = load_literals(&spec)?;
    let allow = load_allowlist(&allowlist_doc)?;

    let tracked = tracked_files(root)?;
    let files: Vec<String> = tracked.iter().filter(|f| in_scope(f)).cloned().collect();

    let in_ci = std::env::var("GITHUB_ACTIONS")
        .map(|v| v == "true")
        .unwrap_or(false);
    let level = match mode {
        Mode::Warn => "warning",
        Mode::Enforce => "error",
    };

    // (file, line, text, tag) — sortable for deterministic output.
    let mut violations: Vec<(String, usize, String, &'static str)> = Vec::new();
    let mut total_matches = 0usize;
    let mut scanned = 0usize;
    let mut conflict = Vec::new();

    for f in &files {
        let content = read_repo_file(root, f)?;
        scanned += 1;
        conflict.extend(conflict_markers::conflict_marker_errors(f, &content));
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

    let readme = read_repo_file(root, README_REL)?;
    let status = read_repo_file(root, STATUS_REL)?;
    let workflow = read_repo_file(root, REPLAY_WORKFLOW_REL)?;
    let player_facing = load_player_facing(root, &tracked)?;
    let front = front_door_errors(&readme, &status, &workflow, &player_facing);
    for e in &front {
        println!("{e}");
        if in_ci {
            println!("::error::{e}");
        }
    }

    conflict.sort();
    for e in &conflict {
        println!("{e}");
        if in_ci {
            println!("::error::{e}");
        }
    }

    if !front.is_empty() {
        bail!(
            "{} front-door documentation error(s) (always enforced)",
            front.len()
        );
    }
    if !conflict.is_empty() {
        bail!(
            "{} git conflict marker(s) in tracked prose (always enforced)",
            conflict.len()
        );
    }
    if should_fail(mode, k) {
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

    fn sample_status() -> String {
        [
            "Local LLMs propose. The deterministic simulation validates and executes.",
            "The game is Aravath. Assets are CC0 only.",
            "under 11 ms per frame on a GTX 1660 Ti-class GPU. The heavy scene measured 3.688 ms. A later sample is 3.117 ms.",
            "planner_pure perception_world gameplay_rng fixed64 combat_resolve",
            "destruction gameplay_qa_aravath_slice nav_reroute causal_bridge scripted_system weaving traversal traversal_deformed worldgen_columns deformed_chunk local_sim greybox_checkpoint quest_ticks persist_replay",
        ]
        .join("\n")
    }

    fn sample_readme() -> String {
        [
            "## In progress",
            "#334 are in progress. #366 are in progress. #383 are in progress",
            "## Next",
        ]
        .join("\n")
    }

    fn sample_workflow() -> String {
        let cases = r#""planner_pure", "perception_world", "gameplay_rng", "fixed64", "combat_resolve", "destruction", "gameplay_qa_aravath_slice", "nav_reroute", "causal_bridge", "scripted_system", "weaving", "traversal", "traversal_deformed", "worldgen_columns", "deformed_chunk", "local_sim", "greybox_checkpoint", "quest_ticks", "persist_replay""#;
        format!("const required = [{cases}];\nrequired = {{{cases}}};\n")
    }

    #[test]
    fn required_lists_must_agree() {
        let lists = required_case_lists(&sample_workflow());
        assert_eq!(lists.len(), 2);
        assert_eq!(lists[0], lists[1]);
        assert_eq!(lists[0].len(), 19);
        assert_eq!(
            lists[0][16..]
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["greybox_checkpoint", "quest_ticks", "persist_replay"]
        );
    }

    #[test]
    fn failed_read_of_project_brief_is_an_error() {
        assert!(is_player_facing("docs/current/PROJECT_BRIEF.md"));
        let dir = std::env::temp_dir().join(format!(
            "aw-doc-lint-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs/current")).unwrap();
        std::fs::create_dir_all(dir.join("docs/campaigns/doc-truth")).unwrap();
        std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
        std::fs::write(
            dir.join(SPEC_REL),
            format!("{VOCAB_A_START}\n{VOCAB_A_END}\n{VOCAB_B_START}\n{VOCAB_B_END}\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join(ALLOWLIST_REL),
            format!("{ALLOWLIST_START}\n{ALLOWLIST_END}\n"),
        )
        .unwrap();
        std::fs::write(dir.join(README_REL), sample_readme()).unwrap();
        std::fs::write(dir.join(STATUS_REL), sample_status()).unwrap();
        std::fs::write(dir.join(REPLAY_WORKFLOW_REL), sample_workflow()).unwrap();
        let brief = dir.join("docs/current/PROJECT_BRIEF.md");
        std::fs::write(&brief, "The game is Aravath.\n").unwrap();
        let init = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&dir)
            .status()
            .unwrap();
        assert!(init.success());
        let add = std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&dir)
            .status()
            .unwrap();
        assert!(add.success());
        std::fs::remove_file(&brief).unwrap();
        let err = lint_repo(&dir, Mode::Warn).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("reading docs/current/PROJECT_BRIEF.md"),
            "{msg}"
        );
        let direct = load_player_facing(&dir, &["docs/current/PROJECT_BRIEF.md".to_string()]);
        let direct_msg = format!("{:#}", direct.unwrap_err());
        assert!(
            direct_msg.contains("reading docs/current/PROJECT_BRIEF.md"),
            "{direct_msg}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn front_door_accepts_a_honest_readme() {
        let errors = front_door_errors(&sample_readme(), &sample_status(), &sample_workflow(), &[]);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn front_door_rejects_a_suffixed_issue_token() {
        let readme = sample_readme().replace("#383 are in progress", "#383foo are in progress");
        let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("missing #383")),
            "{errors:?}"
        );
    }

    #[test]
    fn front_door_rejects_an_extra_in_progress_issue() {
        let readme = sample_readme().replace("#366 are in progress", "#366 #999 are in progress");
        let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("#999")),
            "{errors:?}"
        );
        assert!(
            !errors.iter().any(|error| error.contains("missing #366")),
            "{errors:?}"
        );
    }

    #[test]
    fn front_door_rejects_a_missing_case_and_a_shipped_label() {
        let readme = sample_readme().replace("are in progress", "are shipped");
        let status = sample_status().replace("nav_reroute", "nav");
        let errors = front_door_errors(&readme, &status, &sample_workflow(), &[]);
        assert!(errors.iter().any(|e| e.contains("nav_reroute")));
        assert!(errors.iter().any(|e| e.contains("shipped")));
    }

    #[test]
    fn front_door_rejects_a_frame_figure_on_the_readme() {
        for figure in [
            "3.688 ms",
            "3.117&nbsp;ms",
            "3.117&#32;ms",
            "3.117<br>ms",
            "3.117\nms",
            "3.117\tms",
            "<span>3.117</span> ms",
            "3.117&#x00020;ms",
            "3.117&#x00000020;ms",
            "3.117&#x0000020;ms",
            "3.117&#00000032;ms",
            "3.117&#00032;ms",
            "3.117&thinsp;ms",
            "3.11&#55; ms",
            "3.117 **ms**",
            "3.117 `ms`",
            "3.117&nbsp;**ms**",
            "11 milliseconds",
            "3.117\u{00a0}ms",
            "3.117&#109;&#115;",
            "<div>\n3.117&nbsp;ms\n</div>",
            "3.117 __ms__",
            "3.117 _ms_",
            "3.117 ~~ms~~",
            "<img alt=\"3.117 ms\" src=\"x\">",
            "<img alt=\"3.117&nbsp ms\">",
            "<img alt='3.117 ms' src='x'>",
            "see < 3.117 ms later <img alt=\"x\">",
            "<3.117 ms>",
            "3.117&#x200B;ms",
            "3.117&#8203;ms",
            "3.117&#xfeff;ms",
            "3.117&#x2060;ms",
            "3.117&#x00ad;ms",
            "3.117&zwnj;ms",
            "3.117&zwj;ms",
            "3.117&ZeroWidthSpace;ms",
            "3.117&NewLine;ms",
            "3.117&Tab;ms",
            "3.117&NonBreakingSpace;ms",
            "3.117 millisecond",
            "3.117 msec",
            "3.117&shy;ms",
            "3.117&NoBreak;ms",
            "3.117&hairsp;ms",
            "3.117&#x2063;ms",
            "3.117&numsp;ms",
            "3.117&puncsp;ms",
            "3.117&emsp13;ms",
            "3.117&emsp14;ms",
            "3.117&MediumSpace;ms",
            "3.117&VeryThinSpace;ms",
            "3.117&ThickSpace;ms",
            "3.117&NegativeMediumSpace;ms",
            "3.117&NegativeVeryThinSpace;ms",
            "3.117&ic;ms",
            "3.117&InvisibleComma;ms",
            "3.117&it;ms",
            "3.117&InvisibleTimes;ms",
            "3.117&af;ms",
            "3.117&ApplyFunction;ms",
            "3.117&#x202A;ms",
            "3.117&#x202E;ms",
            "3.117&#x2066;ms",
            "3.117&#x2061;ms",
            "3.117&#x2064;ms",
            "3.117&#x180E;ms",
            "3.117&#xFE0E;ms",
            "3.117&#x034F;ms",
            "３.１１７ ms",
            "３．１１７ ms",
            "3.117&NegativeThickSpace;ms",
            "3.117&NegativeThinSpace;ms",
            "3.117&DotDot;ms",
            "3.117&tdot;ms",
            "3.117&TripleDot;ms",
            "3.117&DownBreve;ms",
            "3.117&#x061C;ms",
            "3.117&#x180B;ms",
            "3.117&#x180C;ms",
            "3.117&#x180D;ms",
            "3.117&#xE0001;ms",
            "3.117\\\nms",
            "3.117\\\r\nms",
            "3.117\u{FF4D}\u{FF53}",
            "3.117\u{08E2}ms",
            "3.117\u{180F}ms",
            "3.117\u{115F}ms",
            "3.117\u{1160}ms",
            "3.117\u{3164}ms",
            "3.117\u{FFA0}ms",
            "<script.foo>3.117 ms</script.foo>",
            "<style></style data-x>3.117 ms",
            "<script>3.117 ms</script>",
            "<style>3.117 ms</style>",
            "<SCRIPT type=\"text/javascript\">3.117 ms</script>",
            "3.117<!--> ms",
            "3.117<!---> ms",
            "3[.117](http://example.com) ms",
            "<div 3.117 ms.",
            "<script 3.117 ms.",
            "<img alt=3.117ms>",
            "<img alt=3.117ms />",
            "<http://example.com/3.117ms>",
            "<IMG ALT=\"3.117 ms\">",
            "<img src=\"x\" alt=\"3.117 ms\">",
        ] {
            let readme = format!("{}\nThe frame is {figure}.\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{figure}: {errors:?}"
            );
        }
    }

    #[test]
    fn front_door_does_not_count_the_gate_sentence_as_the_sample() {
        let status = sample_status().replace(
            "A later sample is 3.117 ms.",
            "aw_doc_lint looks for 3.117 ms.",
        );
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn measured_sample_rejects_a_longer_number_and_a_bare_mention() {
        let longer = sample_status().replace("3.117 ms", "13.117 ms");
        let errors = front_door_errors(&sample_readme(), &longer, &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
        let bare =
            sample_status().replace("A later sample is 3.117 ms.", "Keep 3.117 ms written here.");
        let errors = front_door_errors(&sample_readme(), &bare, &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
        let both = sample_status().replace(
            "A later sample is 3.117 ms.",
            "The median GPU frame is 3.117 ms. aw_doc_lint looks for 3.117 ms.",
        );
        let errors = front_door_errors(&sample_readme(), &both, &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn readme_frame_figure_ignores_a_word_that_only_starts_with_ms() {
        let readme = format!("{}\nShip 3.117 messages.\n", sample_readme());
        let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("frame figure")),
            "{errors:?}"
        );
    }

    #[test]
    fn readme_frame_figure_ignores_hidden_text_and_a_continued_unit() {
        for prose in [
            "Ship 3.117 msecs.",
            "<!-- 3.117 ms -->",
            "<img alt=\"diagram\" src=\"x\">",
            "<img data-alt=\"3.117 ms\" src=\"x\">",
            "see <img alt=\"x\"> later",
            "3.117_ms",
            "3.117~ms",
            "3.117&nbspms",
            "3.117&nbsp ms",
            "3.117&amp;nbsp;ms",
            "`3.117&#x00000020;ms`",
            "`3.117&#x0000020;ms`",
            "`3.117&#00000032;ms`",
            "<img alt=\"3.117&nbspms\">",
            "<div>\n3.117&NBSP ms\n</div>",
            "<div>\n3.117&amp;nbsp;ms\n</div>",
            "<!-- 3.117 ms still hidden -->\nShip messages.",
            "<!-- keep this\n3.117 ms",
            "VISIBLE\n\n<!-- unclosed\n\n3.117 ms",
            "[fig]: http://example.com \"3.117 ms\"",
            "3.117\u{33B3}ms",
            "3.117\u{1734}ms",
        ] {
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
    }

    #[test]
    fn measured_sample_rejects_a_sentence_that_is_not_a_measurement() {
        for sentence in [
            "The unmeasured target is 3.117 ms.",
            "Copy this sample: 3.117 ms.",
            "Not a capture of 3.117 ms.",
            "Never a capture of 3.117 ms.",
            "This cannot be a capture of 3.117 ms.",
            "It wasn't a capture of 3.117 ms.",
            "It wasn\u{2019}t a capture of 3.117 ms.",
            "No capture of 3.117 ms.",
            "Nobody measured 3.117 ms.",
            "Nothing measured 3.117 ms.",
            "It wasn&apos;t a capture of 3.117 ms.",
            "It wasn&rsquo;t a capture of 3.117 ms.",
            "It wasn&#39;t a capture of 3.117 ms.",
            "It wasn&#8217;t a capture of 3.117 ms.",
            "It wasn\u{02BC}t a capture of 3.117 ms.",
            "The capture recorded 3.117 ms.",
            "The median GPU frame is n&#x200B;ot 3.117 ms.",
            "The median GPU frame is n&zwnj;ot 3.117 ms.",
            "The median GPU frame is n&#x00AD;ot 3.117 ms.",
            "The median GPU frame is n&#x2060;ot 3.117 ms.",
            "The median GPU frame is n\u{200B}ot 3.117 ms.",
            "The median GPU frame is &#110;ot 3.117 ms.",
            "The median GPU frame is n&#111;t 3.117 ms.",
            "It wasn\u{FF07}t a measured frame of 3.117 ms.",
            "Neither the median nor the later sample is 3.117 ms.",
            "The median GPU frame is n<b></b>ot 3.117 ms.",
            "The median GPU frame is n*ot* 3.117 ms.",
            "The median GPU frame is n**ot** 3.117 ms.",
            "None measured 3.117 ms.",
            "The median GPU frame is n\u{180F}ot 3.117 ms.",
        ] {
            let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("3.117 ms")),
                "{sentence}: {errors:?}"
            );
        }
        for sentence in [
            "A later sample is 3.117 ms.",
            "A later sample was 3.117 ms.",
            "The median GPU frame is 3.117 ms.",
            "The heavy scene measured a frame of 3.117 ms.",
            "The median GPU frame is nonetheless 3.117 ms.",
        ] {
            let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("3.117 ms")),
                "{sentence}: {errors:?}"
            );
        }
    }

    #[test]
    fn earlier_capture_and_budget_are_tokens() {
        for (from, to, needle) in [
            ("3.688 ms", "13.688 ms", "3.688 ms"),
            ("3.688 ms", "3.688 msec", "3.688 ms"),
            (
                "The heavy scene measured 3.688 ms.",
                "The check names 3.688 ms.",
                "3.688 ms",
            ),
            ("under 11 ms", "under 111 ms", "11 ms"),
            ("under 11 ms", "not under 11 ms", "11 ms"),
            (
                "Local LLMs propose.",
                "It is not true that Local LLMs propose.",
                "Local LLMs propose",
            ),
            (
                "The deterministic simulation validates and executes.",
                "It is not true that the deterministic simulation validates.",
                "deterministic simulation validates",
            ),
        ] {
            let status = sample_status().replace(from, to);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains(needle)),
                "{from} -> {to}: {errors:?}"
            );
        }
    }

    #[test]
    fn capture_noun_is_not_a_measurement() {
        let line1 = "The claims below were checked against `main` at `556a305c`, except the weave-origin notes, which were checked at `43b1b48f` (#388), the #394, #397, #389, and #393 notes, which were checked at `49d289eb`, and the 3.117 ms GTX 1660 Ti Max-Q capture, which was checked at `3855715ea6fd4649a144ee72d2e851956ed1ad3f`";
        let line33 = "Checked against `main` at `556a305c`, except the #388 weave-origin sentences, which were checked at `43b1b48f`, the #394, #397, #389, and #393 bullets, which were checked at `49d289eb`, and the 3.117 ms GTX 1660 Ti Max-Q capture, which was checked at `3855715ea6fd4649a144ee72d2e851956ed1ad3f`";
        for sentence in [line1, line33] {
            let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
            assert!(
                !status_has_measured_sample(&status, "3.117 ms"),
                "{sentence}"
            );
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("3.117 ms")),
                "{sentence}: {errors:?}"
            );
        }
        let kept = sample_status().replace(
            "A later sample is 3.117 ms.",
            "and the 3.117 ms GTX 1660 Ti Max-Q capture, which was checked at `3855715ea6fd4649a144ee72d2e851956ed1ad3f`. The median GPU frame is 3.117 ms.",
        );
        let errors = front_door_errors(&sample_readme(), &kept, &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn html_comment_does_not_supply_front_door_phrases() {
        let hidden = "<!-- 3.688 ms under 11 ms GTX 1660 Ti CC0 Local LLMs propose deterministic simulation validates Aravath. The median GPU frame is 3.117 ms. -->";
        let status = sample_status()
            .replace("3.688 ms", "13.688 ms")
            .replace("under 11 ms", "under 111 ms")
            .replace(
                "A later sample is 3.117 ms.",
                "Keep 13.117 ms written here.",
            )
            .replace("GTX 1660 Ti", "GTX 0000")
            .replace("CC0", "license")
            .replace("Local LLMs propose", "models propose")
            .replace("deterministic simulation validates", "simulation checks")
            .replace("Aravath", "the game");
        let status = format!("{hidden}\n{status}");
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        for needle in [
            "3.688 ms",
            "`11 ms`",
            "3.117 ms",
            "GTX 1660 Ti",
            "CC0",
            "Local LLMs propose",
            "deterministic simulation validates",
            "Aravath",
        ] {
            assert!(
                errors.iter().any(|error| error.contains(needle)),
                "{needle}: {errors:?}"
            );
        }
        assert!(!status_has_measured_sample(
            "<!-- The median GPU frame is 3.117 ms. -->",
            "3.117 ms"
        ));
    }

    #[test]
    fn unclosed_block_comment_hides_front_door_phrases() {
        let good = sample_status();
        for status in [
            format!("<!--\n{good}"),
            format!("VISIBLE_BEFORE\n\n<!-- unclosed\n{good}"),
            format!("VISIBLE_BEFORE\n\n<!-- unclosed\n\n{good}"),
        ] {
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("3.117 ms")),
                "{status:?}: {errors:?}"
            );
            assert!(
                errors.iter().any(|error| error.contains("Aravath")),
                "{status:?}: {errors:?}"
            );
        }
    }

    #[test]
    fn script_and_style_still_supply_front_door_phrases() {
        let good = sample_status();
        for status in [
            format!("<script>\n{good}\n</script>"),
            format!("<style type=\"text/css\">\n{good}\n</style>"),
            format!("<script>\n{good}"),
            format!("<script.foo>\n{good}\n</script.foo>"),
            good.replace(
                "A later sample is 3.117 ms.",
                "<style></style data-x>The median GPU frame is 3.117 ms.",
            ),
        ] {
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(errors.is_empty(), "{status:?}: {errors:?}");
        }
    }

    #[test]
    fn replay_name_inside_a_comment_does_not_count() {
        let status = format!(
            "<!-- nav_reroute -->\n{}",
            sample_status().replace("nav_reroute", "nav_removed")
        );
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("nav_reroute")),
            "{errors:?}"
        );
    }

    #[test]
    fn unicode_gaps_and_ignorable_letters_are_dropped() {
        assert_eq!(unicode_gaps::GAPS.len(), 439);
        assert!(unicode_gaps::is_gap('\u{08E2}'));
        assert!(unicode_gaps::is_gap('\u{180F}'));
        assert!(!unicode_gaps::is_gap('\u{115F}'));
        assert!(!unicode_gaps::is_gap('\u{061C}'));
        for ch in [
            '\u{08E2}', '\u{180F}', '\u{115F}', '\u{1160}', '\u{3164}', '\u{FFA0}', '\u{061C}',
        ] {
            assert!(is_invisible_frame_char(ch), "U+{:04X}", u32::from(ch));
        }
        assert!(!is_invisible_frame_char('\u{33B3}'));
        assert!(!is_invisible_frame_char('\u{00A0}'));
        assert!(!is_invisible_frame_char('\u{1734}'));
    }

    #[test]
    fn gate_docs_name_the_apostrophe_and_the_crate() {
        let workflow = include_str!("../../../.github/workflows/doc-truth-lint.yml");
        assert!(!workflow.contains("anyhow + clap"), "{workflow}");
        assert!(workflow.contains("unicode_categories"));
        let spec = include_str!("../../../docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md");
        assert!(spec.contains("U+2018"));
    }

    #[test]
    fn negated_sentence_does_not_satisfy_cc0_gpu_or_aravath() {
        for (from, to, needle) in [
            ("Assets are CC0 only.", "Assets are not CC0.", "CC0"),
            ("GTX 1660 Ti", "not a GTX 1660 Ti", "GTX 1660 Ti"),
            (
                "The game is Aravath.",
                "The game is not Aravath.",
                "Aravath",
            ),
        ] {
            let status = sample_status().replace(from, to);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains(needle)),
                "{from} -> {to}: {errors:?}"
            );
        }
    }

    #[test]
    fn negation_words_stay_whole_words() {
        assert!(!negated_measurement("the note is normal"));
        assert!(!negated_measurement("unmeasured"));
        assert!(!negated_measurement("nonetheless"));
        assert!(negated_measurement("neither nor"));
        assert!(negated_measurement("not"));
        assert!(negated_measurement("none"));
        assert!(negated_measurement("wasn't"));
    }

    #[test]
    fn comment_apostrophe_does_not_hide_the_next_figure() {
        for prose in [
            "<!-- Don't edit this section -->\nThe frame is 3.117 ms.",
            "<!-- \"keep this -->\nThe frame is 3.117 ms.",
            "The frame is 3.117 ms.\n<!-- keep this",
        ] {
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
    }

    #[test]
    fn autolink_text_can_negate_a_measurement() {
        let status = sample_status().replace(
            "A later sample is 3.117 ms.",
            "The median GPU frame is <https://example.com/not> 3.117 ms.",
        );
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn comment_inside_code_still_negates_a_measurement() {
        let inline = sample_status().replace(
            "A later sample is 3.117 ms.",
            "The median GPU frame is `<!-- not -->` 3.117 ms.",
        );
        let fenced = format!(
            "{}\n```\nThe median GPU frame is <!-- not --> 3.117 ms.\n```\n",
            sample_status().replace("A later sample is 3.117 ms.", "See the fence below.")
        );
        for status in [inline, fenced] {
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("3.117 ms")),
                "{status:?}: {errors:?}"
            );
        }
    }

    #[test]
    fn link_labels_join_into_a_negation() {
        for sentence in [
            "The median GPU frame is n[ot](http://example.com) 3.117 ms.",
            "The median GPU frame is n[](http://example.com)ot 3.117 ms.",
            "The median GPU frame is n[ot][1] 3.117 ms.\n\n[1]: http://example.com",
        ] {
            let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("3.117 ms")),
                "{sentence}: {errors:?}"
            );
        }
    }

    #[test]
    fn link_reference_definition_does_not_supply_the_sample() {
        let status = sample_status().replace(
            "A later sample is 3.117 ms.",
            "See the note.\n\n[note]: http://example.com \"The median GPU frame is 3.117 ms.\"\n\nEND",
        );
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn hanunoo_pamudpod_stays_visible() {
        let status = sample_status().replace(
            "A later sample is 3.117 ms.",
            "The median GPU frame is n\u{1734}ot 3.117 ms.",
        );
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn br_inserts_a_break_between_n_and_ot() {
        let status = sample_status().replace(
            "A later sample is 3.117 ms.",
            "The median GPU frame is n<br>ot 3.117 ms.",
        );
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn front_door_accepts_the_live_pages() {
        let status = include_str!("../../../docs/current/ENGINE_STATUS.md");
        let readme = include_str!("../../../README.md");
        let workflow = include_str!("../../../.github/workflows/determinism-replay.yml");
        let errors = front_door_errors(readme, status, workflow, &[]);
        assert!(errors.is_empty(), "{errors:?}");
        let stripped = status
            .replace(
                "The median GPU frame is 3.117 ms.",
                "The median GPU frame is dated below.",
            )
            .replace(
                "recorded a median GPU frame of 3.117 ms",
                "recorded a median GPU frame in that note",
            );
        let errors = front_door_errors(readme, &stripped, workflow, &[]);
        assert!(
            errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn retired_title_allows_crate_names_and_the_archive_directory() {
        let text =
            "see veilweaver_demo and archive/projects/veilweaver/ only\nThe Veilweaver game\n";
        let hits = retired_title_hits("docs/guides/x.md", text);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].contains(":2:"));
    }

    #[test]
    fn retired_title_lookbehind_is_byte_safe() {
        let hits = retired_title_hits("docs/guides/x.md", "\u{20AC}123456 Veilweaver");
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].contains("docs/guides/x.md:1:"), "{hits:?}");
        assert!(hits[0].contains("retired game title"), "{hits:?}");

        let path = retired_title_hits("docs/guides/x.md", "\u{20AC}projects/veilweaver");
        assert!(path.is_empty(), "{path:?}");

        assert!(retired_title_hits("docs/guides/x.md", "projects/veilweaver").is_empty());
        assert!(retired_title_hits("docs/guides/x.md", "Projects/veilweaver").is_empty());
        assert!(retired_title_hits("docs/guides/x.md", "archive/projects/veilweaver/").is_empty());
        assert!(retired_title_hits("docs/guides/x.md", "veilweaver_demo").is_empty());
        assert!(retired_title_hits("docs/guides/x.md", "veilweaver_").is_empty());
        let game = retired_title_hits("docs/guides/x.md", "The Veilweaver game");
        assert_eq!(game.len(), 1, "{game:?}");
        assert!(game[0].contains(":1:"), "{game:?}");
        let bare = retired_title_hits("README.md", "Veilweaver");
        assert_eq!(bare.len(), 1, "{bare:?}");
        assert!(bare[0].starts_with("README.md:1:"), "{bare:?}");
    }

    #[test]
    fn phrase_token_search_resumes_on_a_char_boundary() {
        assert!(!contains_phrase_token("a\u{00E9}x", "\u{00E9}"));
        assert!(!contains_phrase_token("a\u{00E9}", "\u{00E9}"));
        assert!(contains_phrase_token("a\u{00E9}\u{00E9}", "\u{00E9}"));
        assert!(contains_phrase_token("\u{00E9}", "\u{00E9}"));
        assert!(contains_phrase_token("x \u{00E9} y", "\u{00E9}"));
        assert!(contains_phrase_token(
            "The median GPU frame is 3.117 ms.",
            "median"
        ));
        assert!(contains_phrase_token("measured", "measured"));
        assert!(!contains_phrase_token("unmeasured", "measured"));
        assert!(contains_phrase_token("The sample was 3.117 ms.", "was"));
        assert!(!contains_phrase_token("unwashed", "was"));
    }

    #[test]
    fn second_entity_decode_does_not_drop_not_or_invent_a_space() {
        for sentence in [
            "The median GPU frame is &amp;not; 3.117 ms.",
            "The median GPU frame is \\&not; 3.117 ms.",
            "The median GPU frame is &not 3.117 ms.",
        ] {
            let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("3.117 ms")),
                "{sentence}: {errors:?}"
            );
        }
        for prose in ["3.117&amp;nbsp;ms", "3.117&nbsp ms"] {
            let readme = format!("{}\nThe frame is {prose}.\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
        for prose in [
            "<div>\n3.117&#x00000020;ms\n</div>",
            "<div>\n3.117&#00000032;ms\n</div>",
        ] {
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
    }

    #[test]
    fn referenced_footnote_is_scanned_and_an_unreferenced_one_is_not() {
        let referenced = format!(
            "{}\nSee the note[^1].\n\n[^1]: http://example.com \"3.117 ms\"\n",
            sample_readme()
        );
        let errors = front_door_errors(&referenced, &sample_status(), &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("frame figure")),
            "{errors:?}"
        );
        let unreferenced = format!(
            "{}\n[^1]: http://example.com \"3.117 ms\"\n",
            sample_readme()
        );
        let errors = front_door_errors(&unreferenced, &sample_status(), &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("frame figure")),
            "{errors:?}"
        );
    }

    #[test]
    fn retired_title_and_in_progress_words_use_the_render() {
        for prose in ["The Veil<!-- -->weaver game", "Veil<b></b>weaver"] {
            let errors = front_door_errors(
                &sample_readme(),
                &sample_status(),
                &sample_workflow(),
                &[("docs/guides/x.md".to_string(), prose.to_string())],
            );
            assert!(
                errors
                    .iter()
                    .any(|error| error.contains("retired game title")),
                "{prose}: {errors:?}"
            );
        }
        let allowed = "see veilweaver_demo and projects/veilweaver only";
        let errors = front_door_errors(
            &sample_readme(),
            &sample_status(),
            &sample_workflow(),
            &[("docs/guides/x.md".to_string(), allowed.to_string())],
        );
        assert!(
            !errors
                .iter()
                .any(|error| error.contains("retired game title")),
            "{errors:?}"
        );
        let readme = sample_readme().replace(
            "#334 are in progress. #366 are in progress. #383 are in progress",
            "#334 are ship<!-- -->ped. #366 are land<b></b>ed. #383 are mer<!-- -->ged",
        );
        let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
        for word in ["shipped", "landed", "merged"] {
            assert!(
                errors.iter().any(|error| error.contains(word)),
                "{word}: {errors:?}"
            );
        }
    }

    #[test]
    fn long_numeric_refs_in_text_are_spaces_and_code_keeps_them() {
        for figure in [
            "3.117&#x00000020;ms",
            "3.117&#x0000020;ms",
            "3.117&#00000032;ms",
        ] {
            let readme = format!("{}\nThe frame is {figure}.\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{figure}: {errors:?}"
            );
        }
        for prose in [
            "The frame is `3.117&#x00000020;ms`.",
            "The frame is `3.117&#x0000020;ms`.",
            "The frame is `3.117&#00000032;ms`.",
            "```\n3.117&#x00000020;ms\n```",
            "```\n3.117&#x0000020;ms\n```",
            "```\n3.117&#00000032;ms\n```",
        ] {
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
    }

    #[test]
    fn legacy_names_without_a_semicolon_decode_in_html_and_alt_only() {
        for prose in [
            "<div>\n3.117&nbsp ms\n</div>",
            "<div>\n3.117&nbspms\n</div>",
            "<div>3.117&nbsp ms</div>",
            "<div>3.117&nbspms</div>",
            "<img alt=\"3.117&nbsp ms\">",
        ] {
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
        for prose in [
            "3.117&nbsp ms",
            "3.117&nbspms",
            "<img alt=\"3.117&nbspms\">",
            "<div>\n3.117&NBSP ms\n</div>",
        ] {
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
    }

    #[test]
    fn retired_title_and_in_progress_words_fold_like_the_frame_gate() {
        for prose in [
            "The Veil\u{200B}weaver game",
            "The Veil\u{200D}weaver game",
            "The Veil\u{00AD}weaver game",
            "The \u{FF36}\u{FF45}\u{FF49}\u{FF4C}\u{FF57}\u{FF45}\u{FF41}\u{FF56}\u{FF45}\u{FF52} game",
        ] {
            let errors = front_door_errors(
                &sample_readme(),
                &sample_status(),
                &sample_workflow(),
                &[("docs/guides/x.md".to_string(), prose.to_string())],
            );
            assert!(
                errors
                    .iter()
                    .any(|error| error.contains("retired game title")),
                "{prose}: {errors:?}"
            );
        }
        for (word, label) in [
            ("shi\u{200B}pped", "shipped"),
            ("land\u{200D}ed", "landed"),
            ("mer\u{00AD}ged", "merged"),
            (
                "\u{FF53}\u{FF48}\u{FF49}\u{FF50}\u{FF50}\u{FF45}\u{FF44}",
                "shipped",
            ),
        ] {
            let readme = sample_readme().replace("in progress.", &format!("{word}."));
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains(label)),
                "{word}: {errors:?}"
            );
        }
    }

    #[test]
    fn in_progress_section_ends_on_a_rendered_heading() {
        let fenced = "\
## In progress
#334 are in progress. #366 are in progress. #383 are in progress.
```
## Docs
```
#999 still in the section
### Not a heading
## Docs
";
        let errors = front_door_errors(fenced, &sample_status(), &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("#999")),
            "{errors:?}"
        );
        let commented = "\
## In progress
#334 are in progress. #366 are in progress. #383 are in progress.
<!--
## Docs
-->
#<!-- -->999
## Docs
";
        let errors = front_door_errors(commented, &sample_status(), &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("#999")),
            "{errors:?}"
        );
        assert!(
            !errors.iter().any(|error| error.contains("missing #334")),
            "{errors:?}"
        );
    }

    #[test]
    fn defined_footnote_reference_does_not_join_the_word_around_it() {
        let readme = "\
## In progress
#334 are ship[^1]ped. #366 are land[^note]ed. #383 are in progress.
[^1]: open
[^note]: open
## Docs
";
        let errors = front_door_errors(readme, &sample_status(), &sample_workflow(), &[]);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn in_progress_heading_is_exact_on_setext_and_atx() {
        for readme in [
            "\
Not in progress
---
#334 are in progress. #366 are in progress. #383 are in progress.

## In progress
#999 this shipped
",
            "\
## Not in progress
#334 are in progress. #366 are in progress. #383 are in progress.

## In progress
#999 this shipped
",
        ] {
            let errors = front_door_errors(readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("#999")),
                "{readme}: {errors:?}"
            );
            assert!(
                errors.iter().any(|error| error.contains("shipped")),
                "{readme}: {errors:?}"
            );
        }
        let setext = "\
In progress
---
#334 are in progress. #366 are in progress. #383 are in progress.
## Docs
";
        let errors = front_door_errors(setext, &sample_status(), &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| {
                error.contains("missing") || error.contains("#999") || error.contains("shipped")
            }),
            "{errors:?}"
        );
        for readme in [
            "\
In progress
===
#334 are in progress. #366 are in progress. #383 are in progress.
",
            "\
### In progress
#334 are in progress. #366 are in progress. #383 are in progress.
",
        ] {
            let errors = front_door_errors(readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("missing")),
                "{readme}: {errors:?}"
            );
        }
    }

    #[test]
    fn html_numeric_refs_follow_the_tokenizer() {
        for prose in [
            "<div>\n3.117&#x20ms\n</div>",
            "<div>\n3.117&#xa0ms\n</div>",
            "<div>\n3.117&#x00000020ms\n</div>",
            "<div>\n3.117&#00000032ms\n</div>",
            "<img alt=\"3.117&#xa0ms\">",
            "<img alt=\"3.117&#32ms\">",
        ] {
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
        // `&#32ms` left literal is the figure `32ms`, because `#` is not a
        // word byte. The render check locks the semicolon rule for that spelling.
        for prose in [
            "The frame is 3.117&#32ms.",
            "The frame is 3.117&#x20ms.",
            "The frame is 3.117&#x00000020ms.",
            "The frame is `3.117&#32ms`.",
            "```\n3.117&#32ms\n```",
        ] {
            let entity = if prose.contains("&#x00000020") {
                "&#x00000020"
            } else if prose.contains("&#x20") {
                "&#x20"
            } else {
                "&#32"
            };
            let rendered = rendered::render_visible(prose);
            assert!(
                rendered.contains(entity),
                "markdown text and code keep {entity} when it has no semicolon: {rendered:?}"
            );
        }
        for prose in [
            "The frame is 3.117&#x20ms.",
            "The frame is 3.117&#x00000020ms.",
            "The frame is `3.117&#x20ms`.",
            "```\n3.117&#x20ms\n```",
            "```\n<div>3.117&#x20ms</div>\n```",
            "<div>\n3.117&#0;ms\n</div>",
            "<div>\n3.117&#x0;ms\n</div>",
            "<div>\n3.117&#x85;ms\n</div>",
            "<div>\n3.117&#x85ms\n</div>",
        ] {
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
        for entity in ["&#x92", "&#146"] {
            let status = sample_status().replace(
                "A later sample is 3.117 ms.",
                &format!("\n<div>The median GPU frame isn{entity}t 3.117 ms.</div>"),
            );
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("3.117 ms")),
                "{entity}: {errors:?}"
            );
        }
    }

    #[test]
    fn folded_issue_numbers_count_hidden_marks() {
        for spelling in [
            "#&#x200b;999",
            "#&shy;999",
            "#&zwj;999",
            "\u{FF03}999",
            "#\u{FF19}\u{FF19}\u{FF19}",
        ] {
            let readme = format!(
                "## In progress\n#334 are in progress. #366 are in progress. #383 are in progress.\n{spelling}\n## Docs\n"
            );
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("#999")),
                "{spelling}: {errors:?}"
            );
        }
    }

    #[test]
    fn br_alt_is_a_frame_figure() {
        for prose in [
            "<br alt=\"3.117 ms\">",
            "<div>\n<br alt=\"3.117 ms\">\n</div>",
        ] {
            let rendered = rendered::render_visible(prose);
            assert!(rendered.contains("3.117 ms"), "{prose}: {rendered:?}");
            assert!(
                !rendered.contains("alt="),
                "the attribute is parsed, not copied: {prose}: {rendered:?}"
            );
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
    }

    #[test]
    fn slash_or_ascii_whitespace_starts_an_alt_attribute() {
        for gap in ["/", " ", "\t", "\n", "\r", "\u{000c}"] {
            let prose = format!("<div>\n<img{gap}alt=\"3.117 ms\">\n</div>");
            let rendered = rendered::render_visible(&prose);
            assert!(rendered.contains("3.117 ms"), "{gap:?}: {rendered:?}");
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{gap:?}: {errors:?}"
            );
        }
        let hidden = "<div>\n<imgxalt=\"3.117 ms\">\n</div>";
        let rendered = rendered::render_visible(hidden);
        assert!(
            !rendered.contains("3.117"),
            "a letter is not an attribute boundary: {rendered:?}"
        );
    }

    #[test]
    fn c1_numeric_refs_become_replacement_characters() {
        for n in 0x80u32..=0x9F {
            for prose in [format!("3.117&#x{n:X};ms"), format!("3.117&#{n};ms")] {
                let rendered = rendered::render_visible(&format!("The frame is {prose}."));
                assert!(
                    rendered.contains('\u{FFFD}'),
                    "markdown {prose}: {rendered:?}"
                );
                assert!(
                    !rendered.contains('\u{20AC}'),
                    "markdown does not use the Windows-1252 row: {rendered:?}"
                );
                let readme = format!("{}\nThe frame is {prose}.\n", sample_readme());
                let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
                assert!(
                    !errors.iter().any(|error| error.contains("frame figure")),
                    "markdown {prose}: {errors:?}"
                );
            }
        }
        let padded = rendered::render_visible("The frame is 3.117&#x000080;ms.");
        assert!(padded.contains('\u{FFFD}'), "{padded:?}");
        let code = rendered::render_visible("The frame is `3.117&#x80;ms`.");
        assert!(
            code.contains("&#x80;"),
            "code keeps the reference: {code:?}"
        );
        for (n, ch) in [
            (0x80u32, '\u{20AC}'),
            (0x85, '\u{2026}'),
            (0x92, '\u{2019}'),
            (0x9F, '\u{0178}'),
        ] {
            let rendered = rendered::render_visible(&format!("<div>&#x{n:X};</div>"));
            assert!(rendered.contains(ch), "HTML &#x{n:X}; -> {rendered:?}");
        }
        for n in [0x81u32, 0x8D, 0x8F, 0x90, 0x9D] {
            for prose in [
                format!("<div>\n3.117&#x{n:X};ms\n</div>"),
                format!("<div>\n3.117&#x{n:X}ms\n</div>"),
                format!("<img alt=\"3.117&#{n};ms\">"),
            ] {
                let rendered = rendered::render_visible(&prose);
                assert!(rendered.contains('\u{FFFD}'), "{prose}: {rendered:?}");
                let readme = format!("{}\n{prose}\n", sample_readme());
                let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
                assert!(
                    !errors.iter().any(|error| error.contains("frame figure")),
                    "{prose}: {errors:?}"
                );
            }
        }
    }

    #[test]
    fn c0_and_del_in_markdown_text_stay_replacement_characters() {
        for sentence in [
            "The median GPU frame is n&#1;ot 3.117 ms.",
            "The median GPU frame is n&#x1;ot 3.117 ms.",
            "The median GPU frame is n&#x01;ot 3.117 ms.",
            "The median GPU frame is n\u{0001}ot 3.117 ms.",
            "The median GPU frame is n&#127;ot 3.117 ms.",
            "The median GPU frame is n&#x7F;ot 3.117 ms.",
            "The median GPU frame is n\u{007F}ot 3.117 ms.",
        ] {
            let rendered = rendered::render_visible(sentence);
            assert!(
                rendered.contains('\u{FFFD}'),
                "markdown text keeps U+FFFD: {sentence}: {rendered:?}"
            );
            assert!(
                !rendered.contains("not"),
                "the control does not join the word: {sentence}: {rendered:?}"
            );
            let folded = fold_visible_text(&rendered);
            assert!(
                folded.contains('\u{FFFD}'),
                "the fold keeps U+FFFD: {sentence}: {folded:?}"
            );
            let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("3.117 ms")),
                "{sentence}: {errors:?}"
            );
        }
        for heading in [
            "## &#1;\n",
            "## &#x1;\n",
            "## \u{0001}\n",
            "## &#127;\n",
            "## &#x7F;\n",
            "## \u{007F}\n",
            "## &#xB;\n",
        ] {
            let rendered = rendered::render_visible(heading);
            assert!(
                rendered.contains('\u{FFFD}'),
                "the heading is not empty: {heading:?}: {rendered:?}"
            );
            let readme = format!(
                "## In progress\n#334 are in progress. #366 are in progress. #383 are in progress.\n{heading}#999 this shipped\n"
            );
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| {
                    error.contains("#999") || error.contains("shipped") || error.contains("missing")
                }),
                "{heading:?}: {errors:?}"
            );
        }
        for prose in [
            "3.117&#1;ms",
            "3.117&#x1;ms",
            "3.117\u{0001}ms",
            "3.117&#xB;ms",
            "3.117&#11;ms",
            "3.117\u{000B}ms",
            "3.117&#127;ms",
            "3.117&#x7F;ms",
            "3.117\u{007F}ms",
        ] {
            let rendered = rendered::render_visible(&format!("The frame is {prose}."));
            assert!(
                rendered.contains('\u{FFFD}'),
                "U+000B is not whitespace: {prose}: {rendered:?}"
            );
            let readme = format!("{}\nThe frame is {prose}.\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
        for prose in [
            "3.117&#9;ms",
            "3.117&#x9;ms",
            "3.117\tms",
            "3.117&#10;ms",
            "3.117&#xA;ms",
            "3.117&#12;ms",
            "3.117&#xC;ms",
            "3.117&#x0C;ms",
            "3.117\u{000C}ms",
            "3.117&#13;ms",
            "3.117&#xD;ms",
        ] {
            let rendered = rendered::render_visible(&format!("The frame is {prose}."));
            assert!(
                !rendered.contains('\u{FFFD}'),
                "tab, line feed, form feed, and carriage return stay: {prose}: {rendered:?}"
            );
            let kept = if prose.contains('\t') || prose.contains("&#9;") || prose.contains("&#x9;")
            {
                '\t'
            } else if prose.contains("&#10;") || prose.contains("&#xA;") {
                '\n'
            } else if prose.contains("&#13;") || prose.contains("&#xD;") {
                '\r'
            } else {
                '\u{000C}'
            };
            assert!(
                rendered.contains(kept),
                "the whitespace scalar stays: {prose}: {rendered:?}"
            );
            let readme = format!("{}\nThe frame is {prose}.\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
        for heading in ["## &#12;\n", "## &#xC;\n", "## &#x0C;\n"] {
            let rendered = rendered::render_visible(heading);
            assert!(
                rendered.contains('\u{000C}'),
                "a form-feed heading stays whitespace: {heading:?}: {rendered:?}"
            );
        }
        for heading in ["## &#12;\n", "## &#xC;\n", "## &#x0C;\n", "## \u{000C}\n"] {
            let readme = format!(
                "## In progress\n#334 are in progress. #366 are in progress. #383 are in progress.\n{heading}#999 this shipped\n"
            );
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("#999")),
                "{heading:?}: {errors:?}"
            );
            assert!(
                errors.iter().any(|error| error.contains("shipped")),
                "{heading:?}: {errors:?}"
            );
        }
        let spaced = "The median GPU frame is n&#12;ot 3.117 ms.";
        let status = sample_status().replace("A later sample is 3.117 ms.", spaced);
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("3.117 ms")),
            "form feed does not join not: {errors:?}"
        );
        let code = rendered::render_visible("The frame is `n&#1;ot` and `3.117&#xB;ms`.");
        assert!(
            code.contains("&#1;") && code.contains("&#xB;"),
            "code keeps the reference: {code:?}"
        );
    }

    #[test]
    fn html_c0_and_del_become_replacement_characters() {
        for sentence in [
            "<div>The median GPU frame is n&#1;ot 3.117 ms.</div>",
            "<div>The median GPU frame is n&#x1;ot 3.117 ms.</div>",
            "<div>The median GPU frame is n&#127;ot 3.117 ms.</div>",
            "<div>The median GPU frame is n&#x7F;ot 3.117 ms.</div>",
        ] {
            let rendered = rendered::render_visible(sentence);
            assert!(rendered.contains('\u{FFFD}'), "{sentence}: {rendered:?}");
            assert!(!rendered.contains("not"), "{sentence}: {rendered:?}");
            let folded = fold_visible_text(&rendered);
            assert!(folded.contains('\u{FFFD}'), "{sentence}: {folded:?}");
            let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("3.117 ms")),
                "{sentence}: {errors:?}"
            );
        }
        let readme = "\
## In progress
#334 are in progress. #366 are in progress. #383 are in progress.
<div>ship&#1;ped</div>
## Docs
";
        let errors = front_door_errors(readme, &sample_status(), &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("shipped")),
            "{errors:?}"
        );
        for prose in [
            "<div>\n3.117&#1;ms\n</div>",
            "<div>\n3.117&#xB;ms\n</div>",
            "<div>\n3.117&#11;ms\n</div>",
            "<div>\n3.117&#127;ms\n</div>",
        ] {
            let rendered = rendered::render_visible(prose);
            assert!(rendered.contains('\u{FFFD}'), "{prose}: {rendered:?}");
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
        for prose in [
            "<div>\n3.117&#9;ms\n</div>",
            "<div>\n3.117&#10;ms\n</div>",
            "<div>\n3.117&#12;ms\n</div>",
            "<div>\n3.117&#xC;ms\n</div>",
            "<div>\n3.117&#13;ms\n</div>",
        ] {
            let rendered = rendered::render_visible(prose);
            assert!(!rendered.contains('\u{FFFD}'), "{prose}: {rendered:?}");
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose}: {errors:?}"
            );
        }
        for sentence in [
            "<div>The median GPU frame is n\u{0000}ot 3.117 ms.</div>",
            "<div>The median GPU frame is n\u{0001}ot 3.117 ms.</div>",
            "<div>The median GPU frame is n\u{000B}ot 3.117 ms.</div>",
            "<div>The median GPU frame is n\u{007F}ot 3.117 ms.</div>",
            "<div>The median GPU frame is n\u{0085}ot 3.117 ms.</div>",
            "<div>The median GPU frame is n\u{0092}ot 3.117 ms.</div>",
        ] {
            let rendered = rendered::render_visible(sentence);
            assert!(rendered.contains('\u{FFFD}'), "{sentence:?}: {rendered:?}");
            assert!(
                !rendered.contains('\u{2026}') && !rendered.contains('\u{2019}'),
                "a raw C1 scalar is not Windows-1252: {sentence:?}: {rendered:?}"
            );
            let folded = fold_visible_text(&rendered);
            assert!(folded.contains('\u{FFFD}'), "{sentence:?}: {folded:?}");
            let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
            let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("3.117 ms")),
                "{sentence:?}: {errors:?}"
            );
        }
        let shipped = "\
## In progress
#334 are in progress. #366 are in progress. #383 are in progress.
<div>ship\u{0001}ped</div>
## Docs
";
        let errors = front_door_errors(shipped, &sample_status(), &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("shipped")),
            "{errors:?}"
        );
        for prose in [
            "<div>3.117\u{0001}ms</div>",
            "<div>3.117\u{000B}ms</div>",
            "<div><img alt=\"3.117\u{0001}ms\"></div>",
            "<div><img alt=\"3.117\u{000B}ms\"></div>",
        ] {
            let rendered = rendered::render_visible(prose);
            assert!(rendered.contains('\u{FFFD}'), "{prose:?}: {rendered:?}");
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                !errors.iter().any(|error| error.contains("frame figure")),
                "{prose:?}: {errors:?}"
            );
        }
        for prose in [
            "<div>3.117\u{000C}ms</div>",
            "<div><img alt=\"3.117\u{000C}ms\"></div>",
        ] {
            let rendered = rendered::render_visible(prose);
            assert!(rendered.contains('\u{000C}'), "{prose:?}: {rendered:?}");
            assert!(!rendered.contains('\u{FFFD}'), "{prose:?}: {rendered:?}");
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose:?}: {errors:?}"
            );
        }
    }

    #[test]
    fn bang_comment_close_keeps_the_following_text() {
        for prose in [
            "<!-- hidden --!>\n\nThe frame is 3.117 ms.",
            "<!-- hidden --!> The frame is 3.117 ms.",
            "<div><!-- hidden --!> The frame is 3.117 ms.</div>",
        ] {
            let rendered = rendered::render_visible(prose);
            assert!(rendered.contains("3.117 ms"), "{prose:?}: {rendered:?}");
            assert!(!rendered.contains("hidden"), "{prose:?}: {rendered:?}");
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose:?}: {errors:?}"
            );
        }
        let kept = sample_status().replace(
            "A later sample is 3.117 ms.",
            "<!-- aside --!>\nA later sample is 3.117 ms.",
        );
        let errors = front_door_errors(&sample_readme(), &kept, &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("3.117 ms")),
            "{errors:?}"
        );
    }

    #[test]
    fn link_image_and_html_titles_are_scanned() {
        for prose in [
            "[bench](http://example.com \"3.117 ms\")",
            "![bench](http://example.com '3.117 ms')",
            "[bench](http://example.com (3.117 ms))",
            "<a title=\"3.117 ms\">bench</a>",
            "<img title=\"3.117 ms\">",
            "<div title='3.117 ms'></div>",
            "[bench][id]\n\n[id]: http://example.com \"3.117 ms\"",
        ] {
            let rendered = rendered::render_visible(prose);
            assert!(rendered.contains("3.117 ms"), "{prose:?}: {rendered:?}");
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose:?}: {errors:?}"
            );
        }
        let unused = "[id]: http://example.com \"3.117 ms\"";
        let rendered = rendered::render_visible(unused);
        assert!(
            !rendered.contains("3.117"),
            "an unused reference definition stays omitted: {rendered:?}"
        );
        let readme = format!("{}\n{unused}\n", sample_readme());
        let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
        assert!(
            !errors.iter().any(|error| error.contains("frame figure")),
            "{errors:?}"
        );
        let titled = "\
## In progress
#334 are in progress. #366 are in progress. #383 are in progress.
[pulls](http://example.com \"shipped\")
## Docs
";
        let errors = front_door_errors(titled, &sample_status(), &sample_workflow(), &[]);
        assert!(
            errors.iter().any(|error| error.contains("shipped")),
            "{errors:?}"
        );
        let guide = "[x](http://example.com \"The Veilweaver game\")";
        let errors = front_door_errors(
            &sample_readme(),
            &sample_status(),
            &sample_workflow(),
            &[("docs/guides/x.md".to_string(), guide.to_string())],
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("retired game title")),
            "{errors:?}"
        );
    }

    #[test]
    fn titles_land_at_the_end_of_the_block() {
        for prose in [
            "Frame 3 <abbr title=\"x\">ms</abbr>",
            "Frame 3 [ms](https://a.b \"x\")",
            "Frame 3 ![ms](x.png \"x\")",
        ] {
            let folded = fold_visible_text(&rendered::render_visible(prose));
            assert!(
                folded.lines().any(|line| line.contains("Frame 3 ms")),
                "{prose:?}: {folded:?}"
            );
            assert!(
                folded.lines().any(|line| line.trim() == "x"),
                "{prose:?}: {folded:?}"
            );
            assert!(
                !folded
                    .lines()
                    .any(|line| line.contains("Frame 3") && line.contains('x')),
                "{prose:?}: {folded:?}"
            );
            let readme = format!("{}\n{prose}\n", sample_readme());
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("frame figure")),
                "{prose:?}: {errors:?}"
            );
        }
        let sentence = "The median GPU frame is [3.117 ms](https://a.b \"capture log\").";
        let folded = fold_visible_text(&rendered::render_visible(sentence));
        assert!(
            folded
                .lines()
                .any(|line| line.contains("The median GPU frame is 3.117 ms.")),
            "{folded:?}"
        );
        assert!(
            folded.lines().any(|line| line.trim() == "capture log"),
            "{folded:?}"
        );
        assert!(
            !folded
                .lines()
                .any(|line| line.contains("3.117 ms") && line.contains("capture")),
            "{folded:?}"
        );
        let status = sample_status().replace("A later sample is 3.117 ms.", sentence);
        let errors = front_door_errors(&sample_readme(), &status, &sample_workflow(), &[]);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn in_progress_section_finishes_the_next_heading() {
        let head = "\
## In progress
#334 are in progress. #366 are in progress. #383 are in progress.
";
        let tail = "\
#999 still here
## Docs
";
        let setext_br: Vec<_> = pulldown_cmark::Parser::new("In<br>progress\n---\n").collect();
        assert!(
            setext_br.iter().any(|event| matches!(
                event,
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading {
                    level: pulldown_cmark::HeadingLevel::H2,
                    ..
                })
            )),
            "setext In<br>progress is an h2: {setext_br:?}"
        );
        for heading in [
            "## In progress\n",
            "## In  progress\n",
            "## In\u{00A0}progress\n",
            "## In\u{2003}progress\n",
            "## In\tprogress\n",
            "## \u{200B}\n",
            "In<br>progress\n---\n",
        ] {
            let readme = format!("{head}\n{heading}\n{tail}");
            let errors = front_door_errors(&readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("#999")),
                "{heading:?}: {errors:?}"
            );
            assert!(
                !errors.iter().any(|error| error.contains("missing #334")),
                "{heading:?}: {errors:?}"
            );
        }
        for readme in [
            "\
## In  progress
#334 are in progress. #366 are in progress. #383 are in progress.
",
            "\
In<br>progress
---
#334 are in progress. #366 are in progress. #383 are in progress.
",
        ] {
            let errors = front_door_errors(readme, &sample_status(), &sample_workflow(), &[]);
            assert!(
                errors.iter().any(|error| error.contains("missing")),
                "collapsed spaces do not open the section: {readme:?}: {errors:?}"
            );
        }
    }
}
