# Contributing to AstraWeave

Agents follow [AGENTS.md](AGENTS.md). This file is the human setup guide.

Thank you for your interest in helping advance the AstraWeave AI-Native Gaming Engine. This document explains how to set up your environment, propose changes, and collaborate effectively with the team.

## Getting Started

- Ensure you are using the pinned toolchain in `rust-toolchain.toml` (Rust 1.89.0).
- Determinism changes must keep the nineteen required replay cases identical across Linux, Windows, and macOS. The list is in `.github/workflows/determinism-replay.yml` and [engine status](docs/current/ENGINE_STATUS.md). `local_sim` is the sixteenth. `greybox_checkpoint`, `quest_ticks`, and `persist_replay` are the seventeenth through nineteenth.
- Local LLMs propose. The deterministic simulation validates and executes.
- CodeRabbit reviews pull requests, including drafts, using `.coderabbit.yaml`. It requests changes when a review has actionable comments. Push updates are reviewed again. Generated files, golden hashes, `docs/architecture/workspace_map.html`, and `assets/` are outside that review.
- Player-facing text says Aravath. Assets added to the tree are CC0 only.
- #216 is on `main`. The measured frame is in `docs/current/ENGINE_STATUS.md`. Do not publish a new frame number without its commit SHA and adapter. Local field #330 is on `main` (`astraweave-localsim`, required case `local_sim`). #253 and #336 are closed. `gravity.rs` is deleted. Weaving #252, the Aravath greybox #205, traversal #244, terrain columns #215, CI burndown #217, the greybox tokio graph #327, the greybox combat slice #275, the off-tick AI change #331, and the greybox showcase #329 are on `main`. The greybox golden is `b6a9d5a2c5da28a6`. #326 is closed. #340 closed #273 and #338. #333 closed #218 and took the terrain and AI rows off the pull-request skip list. #254, #242, #314, #304, and #274 stay open. `AIArbiter` and `GoapPlanner` still exist and stay off the deterministic tick.
- Install required native dependencies for `wgpu`, `rapier`, and `ffmpeg` (refer to platform notes in `docs/masters/MASTER_ROADMAP.md`).
- Clone the repository and run `cargo fetch` to prime the workspace.

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
rustup show
```

## Development Workflow

1. Create an issue or comment on an existing one to signal intent.
2. Fork the repository (external contributors) and create a feature branch: `git checkout -b feat/my-change`.
3. Make focused commits that address a single concern.
4. Run the verification checklist (below) before opening a pull request (PR).

## Code Style and Quality

- Format code with `cargo fmt --all`.
- Lint with `cargo clippy --workspace -- -D warnings`.
- Maintain existing patterns, error handling idioms, and module organization.
- Prefer descriptive names; avoid abbreviations unless established in the codebase.
- Do not introduce new unsafe blocks without justification and tests.

## Testing Requirements

- `cargo test --workspace` (or the specific crate you touched).
- Run targeted integration tests when modifying systems with heavy dependencies (e.g., rendering, networking, AI orchestrator).
- For performance-sensitive code, run relevant benchmarks (`cargo bench -p <crate>`).
- Document any skipped tests or platform-specific limitations in the PR description.

## Pull Request Checklist

- [ ] Issue reference included in the PR description (if applicable).
- [ ] Implementation scoped to the stated problem.
- [ ] New public APIs documented in code and, when needed, in `docs/`.
- [ ] Tests added or updated.
- [ ] `cargo fmt`, `cargo clippy`, and required test suites pass locally.

## Commit Guidelines

- Use conventional prefixes when possible (`feat:`, `fix:`, `docs:`, `refactor:`).
- Keep commit messages concise and descriptive.
- Squash commits if they represent iterative fixes to the same change set.

## Issue Reporting

- Use the [issue chooser](https://github.com/lazyxeon/AstraWeave/issues/new/choose) for **Bug report** or **Feature request** templates (blank issues are disabled).
- Include reproduction steps, expected vs. actual results, and logs if available.
- For security-related reports, follow `SECURITY.md` and report through GitHub private vulnerability reporting: https://github.com/lazyxeon/AstraWeave/security/advisories/new

## Communication

- Use a **Feature request** issue or pull-request comments for design proposals and clarifications.
- Keep conversations respectful and aligned with the `CODE_OF_CONDUCT.md`.

We appreciate your contributions—together we are building the future of AI-native game development.

## Revision history

### vNEXT

- Issue chooser and template links; security via private advisories.