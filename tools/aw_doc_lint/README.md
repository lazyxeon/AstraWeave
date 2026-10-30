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

**Markers:** lines starting with `<<<<<<<` or `>>>>>>>` (followed by a space or end of line), `|||||||` diff3 markers, and `=======` only while a conflict block is open. Setext underlines stay allowed.

**Inside fenced code blocks:** markers are reported like prose, except a complete example inside a fence whose info string is exactly `diff`, `text`, or `console` (opening line `` ```diff ``, `` ```text ``, `` ```console `` with a full `<<<<<<<` … `=======` … `>>>>>>>` sequence before the fence closes).

**Fences:** CommonMark rules — up to three spaces of indent; closing run uses the same `` ` `` or `~` character, length ≥ the opener, and only whitespace after the run (no info string on the closing line).

Violations print as `file:line`.

## Tests

```bash
cargo test -p aw_doc_lint
```
