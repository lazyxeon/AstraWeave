# Contributing to AstraWeave

Thank you for your interest in helping advance the AstraWeave AI-Native Gaming Engine. This document explains how to set up your environment, propose changes, and collaborate effectively with the team.

## Getting Started

- Ensure you are using the pinned toolchain in `rust-toolchain.toml` (Rust 1.89.0).
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

- Use GitHub Issues for bug reports and feature requests.
- Include reproduction steps, expected vs. actual results, and logs if available.
- For security-related reports, follow the process in `SECURITY.md`.

## Communication

- Use the discussion board or issue comments for design proposals and clarifications.
- Keep conversations respectful and aligned with the `CODE_OF_CONDUCT.md`.

We appreciate your contributions—together we are building the future of AI-native game development.