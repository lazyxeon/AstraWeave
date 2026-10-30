# aw_doc_lint

Closed-vocabulary documentation-truth lint for the D-series campaign. The spec lives in [`docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md`](../../docs/campaigns/doc-truth/CLOSED_VOCABULARY_LINT.md).

## Running

```bash
cargo run -p aw_doc_lint -- --mode warn
```

`--mode enforce` exits nonzero on un-allowlisted vocabulary hits. Several checks are **always enforced** (nonzero even in `--mode warn`):

- **Front-door** rules on `README.md`, `docs/current/ENGINE_STATUS.md`, replay workflow names, and player-facing retired-title scan (documented under “Front-door check” in the spec).
- **Git conflict markers** in tracked prose (`*.md`, gh-pages `.html` / `_config.yml` in scope): lines starting with `<<<<<<<` or `>>>>>>>` (followed by a space or end of line), `|||||||` diff3 markers, and a `=======` line only when it sits between an opening `<<<<<<<` and a closing `>>>>>>>`. Standalone `=======` setext underlines are ignored. Content inside fenced `` ``` `` blocks is skipped. Violations are reported as `file:line`.

## Tests

```bash
cargo test -p aw_doc_lint
```
