# Archived one-off scripts

These files used to sit at the repository root. They are not the maintained tools in `scripts/`. Read this note before running one.

## Destructive

These rewrite files in place.

- `fix_lore_bible.py`, `fix_lore_pass2.py`, `fix_lore_pass3.py`, and `fix_formatting.py` write `docs/Veilweaver/lore_bible.md`.
- `fix_editor_emojis.py` rewrites Rust sources. It joins `tools/aw_editor/src` onto the script's own directory, so that path is correct only when the script lives at the repository root.

## Hardcoded paths

These embed a Windows temp directory or a personal workspace path. They do not use the current checkout.

- `run_terrain_sweep.py` hardcodes `C:\Users\pv2br\AstraWeave-AI-Native-Gaming-Engine` and writes `terrain_sweep_results` there. Before each shard, `cleanup_temp` deletes every `%TEMP%` directory whose name starts with `cargo-mutants`, then `shutil.rmtree`s all of `C:\temp`. Errors from those deletes are swallowed.
- `run_render_sweep.py` does not embed that personal workspace path. It runs in the current checkout. `cleanup_temp` deletes only `C:\temp\mutants.out`. The cargo-mutants `-o` flag still writes that crate's output under `C:\temp`. The script writes `render_sweep_results/aggregate.json` and `render_sweep_results/summary.txt` in the current directory.
- `analyze_terrain_sweep.py` reads that same Windows workspace's `terrain_sweep_results/aggregate.json`.
- `analyze_misses.py` reads `C:\temp\mutants.out\outcomes.json`.

## Read-only or throwaway

- `verify_targets.py` only reads `docs/Veilweaver/lore_bible.md`.
- `parse_outcomes.py` writes whatever output path you pass.
- `test_mountain.rs` is a standalone `main`. It is not a workspace member.
