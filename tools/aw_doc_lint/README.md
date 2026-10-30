# aw_doc_lint

Closed-vocabulary documentation-truth lint for the D-series campaign. The spec lives in [`docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md`](../../docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md).

## Running

```bash
cargo run -p aw_doc_lint -- --mode warn
```

`--mode enforce` exits nonzero on un-allowlisted vocabulary hits. Several checks are **always enforced** (nonzero even in `--mode warn`):

- **Front-door** rules on `README.md`, `docs/current/ENGINE_STATUS.md`, replay workflow names, and player-facing retired-title scan (documented under “Front-door check” in the spec).
- **Git conflict markers** on all tracked text files (see below). The closed-vocabulary scan keeps its existing narrower scope.

### Conflict-marker scan (always enforced)

**Files:** every `git ls-files` path with a known text extension (`.md`, `.html`, `.yml`, `.json`, `.rs`, `.toml`, …) or a small set of extensionless names (`README`, `LICENSE`, …). Skips `.vs/`, `vendor/`, `node_modules/`, `target/`, invalid UTF-8, and NUL bytes. This includes `docs/archive/` (for example the benchmark dashboard HTML #538 publishes) and other paths excluded from vocabulary lint.

**Markers:** column-0 git conflict lines: `<<<<<<<` / `>>>>>>>` / `|||||||` (marker plus space or end of line). A `=======` line is reported only while a conflict is open (after `<<<<<<<` and before the matching `>>>>>>>`), in prose and in fences; a setext underline or a fenced Markdown heading line of seven equals is not flagged on its own.

**Inside `` ```diff `` / `` ```text `` / `` ```console `` fences only:** the first balanced `<<<<<<<` … `=======` … `>>>>>>>` example in that fence is omitted from the report when the fence closes cleanly. A second `<<<<<<<` while a conflict is still open cancels that exemption, so an earlier unclosed opener is reported. Any later marker in the same fence (including a stray `>>>>>>>`, `|||||||`, a second full conflict, or a trailing `<<<<<<<`) is still reported. A backtick fence whose info string contains `` ` `` is not a CommonMark fence and is scanned like normal lines.

**Fences:** CommonMark rules — up to three spaces of indent; closing run uses the same `` ` `` or `~` character, length ≥ the opener, and only whitespace after the run (no info string on the closing line).

Violations print as `file:line`.

## Tests

```bash
cargo test -p aw_doc_lint
```
