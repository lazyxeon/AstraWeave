# Documentation Audit - Quick Action Checklist

**Priority**: Immediate actions for production readiness  
**Timeline**: 1-2 weeks (40 hours focused work)  
**Impact**: C+ (73/100) → B (82/100) documentation grade

---

## Week 1: Foundation (20 hours)

### Day 1-2: Core Documentation Files (10 hours)

- [ ] **Create CONTRIBUTING.md** (4 hours)
  - [ ] PR submission guidelines (fork, branch, commit conventions)
  - [ ] Code style requirements (rustfmt, clippy)
  - [ ] Testing requirements (unit tests, integration tests, benchmarks)
  - [ ] Documentation standards (doc comments, per-crate READMEs)
  - [ ] Review process (CI checks, code review expectations)
  - Template: https://github.com/nayafia/contributing-template/blob/HEAD/CONTRIBUTING-template.md

- [ ] **Create CODE_OF_CONDUCT.md** (1 hour)
  - [ ] Use Contributor Covenant 2.1: https://www.contributor-covenant.org/
  - [ ] Add enforcement contact (email/Discord)
  - [ ] Add to root directory

- [ ] **Create ARCHITECTURE.md** (5 hours)
  - [ ] Overview: 7-stage execution pipeline (Pre-Sim, Perception, Simulation, AI Planning, Physics, Post-Sim, Presentation)
  - [ ] AI-Native Design: Perception → Reasoning → Planning → Action → Validation
  - [ ] ECS Architecture: Deterministic archetype-based system
  - [ ] Rendering Pipeline: wgpu 25.0.2, PBR + IBL, MegaLights
  - [ ] Diagrams: Mermaid flowcharts (already in README, reference those)
  - [ ] Technology stack table (from README)

### Day 3-4: Version History (10 hours)

- [ ] **Create CHANGELOG.md** (8 hours)
  - [ ] Follow [Keep a Changelog](https://keepachangelog.com/) format
  - [ ] Sections: Added, Changed, Deprecated, Removed, Fixed, Security
  - [ ] Start with v0.4.0 (current version from README)
  - [ ] Document Phase 1-8 changes retroactively:
    - [ ] v0.1.0 - Phase 0: Core ECS foundation
    - [ ] v0.2.0 - Phase 1-4: AI orchestration, determinism
    - [ ] v0.3.0 - Phase 5-7: Coverage improvements, LLM validation
    - [ ] v0.4.0 - Phase 8: Rendering overhaul (Phases 1-8 complete)
  - [ ] Breaking changes: winit 0.29→0.30, wgpu 22→25, egui 0.28→0.32
  - [ ] Migration guide stubs (link to future docs)

- [ ] **Set up Git tagging** (2 hours)
  - [ ] Create git tag for v0.4.0: `git tag -a v0.4.0 -m "Phase 8 Rendering Complete"`
  - [ ] Document versioning policy in CONTRIBUTING.md
  - [ ] Add tag-based release workflow (GitHub Releases)

---

## Week 2: Per-Crate Documentation (20 hours)

### Day 5-7: P0 Crate READMEs (15 hours)

**Template** (use for all crates):
```markdown
# astraweave-{name}

{One-sentence purpose}

## Features

- Feature 1
- Feature 2
- Feature 3

## Quick Example

\`\`\`rust
use astraweave_{name}::*;

fn main() {
    // Minimal working example
}
\`\`\`

## API Reference

See [API Documentation](https://docs.rs/astraweave-{name}) (link to docs.rs when published)

## License

MIT
```

**Priority Order** (2.5 hours each):

- [ ] **astraweave-core** (CRITICAL!)
  - Purpose: Core simulation engine (World, validation, tool sandbox)
  - Features: Deterministic ECS bridge, 37-tool vocabulary, perception system
  - Example: hello_companion integration
  - Key APIs: World, ActionStep, validate_and_execute

- [ ] **astraweave-ai** (P0)
  - Purpose: AI orchestration layer (6 planning modes)
  - Features: GOAP planner, LLM integration, 12,700+ agents @ 60 FPS
  - Example: AI core loop (Classical/BehaviorTree/Utility/LLM/Hybrid/Ensemble)
  - Key APIs: AiArbiter, GoapPlanner, LlmExecutor

- [ ] **astraweave-physics** (P0)
  - Purpose: Rapier3D integration (character controller, rigid bodies, raycasting)
  - Features: 95.07% test coverage, 533 bodies @ 60 FPS, spatial hash optimization
  - Example: Character movement + collision
  - Key APIs: CharacterController, RigidBodySet, SpatialHash

- [ ] **astraweave-nav** (P0)
  - Purpose: Navmesh generation + A* pathfinding
  - Features: 94.66% coverage, 142k queries/sec, 15 winding bugs fixed
  - Example: Pathfinding in complex geometry
  - Key APIs: NavMesh, astar_path, find_cover_positions

- [ ] **astraweave-audio** (P0)
  - Purpose: Spatial audio + dialogue runtime
  - Features: 91.42% coverage, music crossfading, TTS adapter
  - Example: Spatial audio + dialogue playback
  - Key APIs: AudioEngine, DialogueRuntime, VoiceBank

- [ ] **astraweave-behavior** (P0)
  - Purpose: Behavior trees + GOAP + utility AI
  - Features: 94.34% coverage, 97.9% faster GOAP cache hits
  - Example: Behavior tree + GOAP planning
  - Key APIs: BehaviorTree, GoapPlanner, UtilityAI

### Day 8: P1-A Crate READMEs (5 hours)

- [ ] **astraweave-render** (P1-B, but critical)
  - Purpose: wgpu 25 rendering pipeline (PBR + IBL + MegaLights)
  - Features: 63.62% coverage, AAA rendering, 350 tests, 1.2-1.4ms frame time
  - Example: unified_showcase rendering
  - Key APIs: Renderer, MaterialManager, ClusteredForward

---

## Bonus Tasks (If Time Permits)

### Documentation Tools Setup (5 hours)

- [ ] **Install cargo-readme** (15 min)
  ```bash
  cargo install cargo-readme
  ```

- [ ] **Generate per-crate READMEs from lib.rs** (1 hour)
  - Add doc comments to lib.rs for each crate
  - Run `cargo readme > README.md` in each crate directory

- [ ] **Set up cargo-deadlinks** (30 min)
  ```bash
  cargo install cargo-deadlinks
  cargo doc --workspace --no-deps
  cargo deadlinks --check-http
  ```

- [ ] **Set up markdownlint** (30 min)
  - Install: `npm install -g markdownlint-cli`
  - Create `.markdownlint.json` config
  - Add to CI: `markdownlint '**/*.md'`

- [ ] **Create CI documentation check** (2 hours)
  - GitHub Actions workflow: `.github/workflows/docs.yml`
  - Check: All crates have README.md
  - Check: `cargo doc --workspace --no-deps` builds successfully
  - Check: No broken links (cargo-deadlinks)
  - Check: Markdown lint passes

### TODO Tracking (5 hours)

- [ ] **Extract all TODOs to GitHub issues** (3 hours)
  - Script to find all TODO/FIXME comments
  - Create GitHub issues for each (label: "documentation" or "bug")
  - Link issues in code comments: `// TODO: See issue #123`

- [ ] **Prioritize critical TODOs** (2 hours)
  - Create GitHub milestone: "v1.0 Documentation"
  - Assign priority labels:
    - P0 (blocking): 5 critical TODOs (renderer, editor, embeddings)
    - P1 (important): 20 TODOs (features, polish)
    - P2 (nice-to-have): 75 TODOs (future work)

---

## Validation Checklist

### Before Considering Week 1-2 Complete:

- [ ] All 4 root documentation files created (CONTRIBUTING, CHANGELOG, CODE_OF_CONDUCT, ARCHITECTURE)
- [ ] At least 7 crate READMEs created (P0 + astraweave-render)
- [ ] Git tag v0.4.0 created and pushed
- [ ] CHANGELOG.md covers Phases 0-8 retroactively
- [ ] All new Markdown files pass linting (markdownlint)
- [ ] Documentation audit summary updated with new status

### Success Metrics:

**Before** (Current):
- Root docs: 40% (3/7 files)
- Per-crate READMEs: 10.6% (5/47 crates)
- Overall grade: C+ (73/100)

**After** (Week 1-2):
- Root docs: 100% (7/7 files) ✅
- Per-crate READMEs: 25.5% (12/47 crates) ⬆️ +15pp
- Overall grade: B (82/100) ⬆️ +9 points

---

## Next Steps After Week 1-2

### Week 3-4: Usability Improvements (20 hours)

- [ ] Create example index (docs/src/examples/INDEX.md)
- [ ] Add migration guides (winit, wgpu, egui)
- [ ] Expand troubleshooting (Windows GPU, macOS build)
- [ ] Add API examples for P0 crates

### Week 5-6: Remaining Crate READMEs (30 hours)

- [ ] P1-B crates (4 remaining): gameplay, scene, terrain
- [ ] P2 crates (7 total): context, embeddings, persona, prompts, rag
- [ ] P3 crates (30 remaining): all support crates

### Month 2-3: Polish & Automation (40 hours)

- [ ] API doc coverage to 90%+ (cargo doc examples)
- [ ] Example documentation (inline docs for all 27 examples)
- [ ] CI documentation checks (enforce standards)
- [ ] Automated changelog (git-cliff integration)

---

## Resources

### Templates & Tools

- **Contributor Covenant**: https://www.contributor-covenant.org/
- **Keep a Changelog**: https://keepachangelog.com/
- **GitHub Contributing Template**: https://github.com/nayafia/contributing-template
- **cargo-readme**: https://crates.io/crates/cargo-readme
- **cargo-deadlinks**: https://crates.io/crates/cargo-deadlinks
- **markdownlint**: https://github.com/DavidAnson/markdownlint
- **git-cliff**: https://git-cliff.org/ (automated CHANGELOG)

### AstraWeave References

- **Master Roadmap**: docs/current/MASTER_ROADMAP.md v1.23
- **Coverage Report**: docs/current/MASTER_COVERAGE_REPORT.md v1.33
- **Copilot Instructions**: .github/copilot-instructions.md
- **README.md**: Comprehensive overview (494 lines)
- **Documentation Audit**: docs/current/DOCUMENTATION_AUDIT_REPORT.md (this report)

---

## Notes

### Time Estimates

- Foundation (Week 1): 20 hours
  - CONTRIBUTING.md: 4h
  - CODE_OF_CONDUCT.md: 1h
  - ARCHITECTURE.md: 5h
  - CHANGELOG.md: 8h
  - Git tagging: 2h

- Per-Crate READMEs (Week 2): 20 hours
  - P0 crates (6): 15h (2.5h each)
  - astraweave-render: 2.5h
  - Buffer time: 2.5h

**Total**: 40 hours (1 week full-time, 2 weeks half-time)

### Expected Impact

**Documentation Grade**: C+ (73/100) → B (82/100) (+9 points)
**User Experience**: Significant improvement in onboarding
**Contributor Experience**: Clear guidelines for contributions
**Maintenance**: Version tracking and change history established

---

**Status**: Ready to execute  
**Owner**: TBD  
**Deadline**: 2 weeks from start date  
**Tracking**: Create GitHub project board with these tasks
