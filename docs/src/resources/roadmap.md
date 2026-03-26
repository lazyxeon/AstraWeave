# Roadmap

This document outlines the development roadmap for AstraWeave, including current status, planned features, and long-term vision.

## Current Status

**Version**: 0.1.0 (Alpha)

AstraWeave is currently in active development. The core systems are functional but APIs may change before 1.0.

### Stability Matrix

| System | Status | API Stability |
|--------|--------|---------------|
| ECS | Stable | High |
| Rendering | Beta | Medium |
| Physics | Stable | High |
| AI/LLM | Beta | Medium |
| Audio | Stable | High |
| Networking | Alpha | Low |
| Editor | Alpha | Low |

## Recently Completed Milestones

| Milestone | Date | Tests | Highlights |
|-----------|------|-------|------------|
| **Blueprint Zone Editor** | Mar 2026 | 125+ | Polygon zone editor, Replica/Inspired placement, heightmap injection, 3D overlay |
| **Blend Import Pipeline** | Feb 2026 | 97 | .blend scene decomposition, BiomePack bridge, texture processing, editor panel |
| **Veilweaver Vertical Slice** | Feb 2026 | 320 | 5-phase game slice, dialogue, cinematics, boss HUD, determinism validation |
| **Miri Memory Safety** | Feb 2026 | 977 | 4 crates validated, zero undefined behavior |
| **Fluids System (A+)** | Jan 2026 | 4,907 | SPH/FLIP simulation, comprehensive test suite |
| **AI-Orchestrated Terrain** | Dec 2025 | 2,536 | TerrainSolver, NavMesh rebaking, terrain persistence |
| **Rendering Pipeline** | Dec 2025 | 806+ | PBR/IBL, CSM shadows, headless support, post-processing |

## Near-Term (Q1-Q2 2026)

### v0.2.0 - Polish Release

Focus: Stability and developer experience.

- [ ] **API Stabilization**
  - Finalize core ECS APIs
  - Stabilize component derive macros
  - Lock down resource patterns

- [ ] **Documentation**
  - Complete API documentation
  - Add more tutorials
  - Video walkthroughs

- [ ] **Tooling**
  - Improved editor prototype
  - Better debugging tools
  - Asset pipeline improvements

- [ ] **Performance**
  - Rendering optimization
  - AI tick budgeting improvements
  - Memory usage reduction

### v0.3.0 - AI Enhancement

Focus: Advanced AI capabilities.

- [ ] **LLM Improvements**
  - Multi-provider support (OpenAI, Anthropic, local)
  - Streaming responses
  - Function calling standardization
  - Context window management

- [ ] **Behavior Systems**
  - Visual behavior tree editor
  - GOAP improvements
  - Utility AI integration

- [ ] **Memory Systems**
  - Improved episodic memory
  - Semantic search with embeddings
  - Memory persistence

- [ ] **AI Characters**
  - Personality templates
  - Dynamic relationship modeling
  - Emotion simulation

## Mid-Term (Q3-Q4 2026)

### v0.4.0 - Content Creation

Focus: Procedural content and world building.

- [ ] **PCG Framework**
  - Dungeon generation improvements
  - Terrain generation enhancements
  - AI-assisted content creation

- [ ] **World Streaming**
  - Large world support
  - Seamless level streaming
  - Background loading

- [ ] **Quest System**
  - Dynamic quest generation
  - Branching narratives
  - AI-driven story adaptation

### v0.5.0 - Editor & Tools

Focus: Complete development environment.

- [ ] **Visual Editor**
  - Scene editing
  - Prefab system
  - Play-in-editor

- [ ] **Asset Pipeline**
  - Automated texture processing
  - Model optimization
  - Audio processing

- [ ] **Debugging**
  - In-game console
  - Entity inspector
  - Performance profiler

- [ ] **Scripting**
  - Rhai scripting integration
  - Hot-reload for scripts
  - Visual scripting (prototype)

## Long-Term (2027+)

### v1.0.0 - Production Ready

Focus: Stability for production games.

- [ ] **API Freeze**
  - Stable, documented APIs
  - Semantic versioning
  - Deprecation policy

- [ ] **Platform Support**
  - Windows, Linux, macOS (native)
  - WebAssembly
  - Console support (planned)

- [ ] **Performance Guarantees**
  - Documented performance characteristics
  - Benchmark suite
  - Regression testing

- [ ] **Security**
  - Security audit
  - Sandboxed scripting
  - Save file validation

### Future Vision

Features being considered for post-1.0:

- **Console Ports**: PlayStation, Xbox, Nintendo Switch support
- **VR/AR**: Virtual and augmented reality support
- **Cloud Gaming**: Streaming and cloud save integration
- **Multiplayer**: Advanced networking and matchmaking
- **Marketplace**: Asset and plugin marketplace
- **AI Cloud**: Optional cloud-based AI processing
- **Mobile**: iOS and Android support

## Feature Requests

### How to Request Features

1. **Check existing requests**: Search [GitHub Issues](https://github.com/astraweave/astraweave/issues)
2. **Create a discussion**: Use [GitHub Discussions](https://github.com/astraweave/astraweave/discussions) for initial feedback
3. **Submit formal request**: Create an issue with the `feature-request` label

### Prioritization Criteria

Features are prioritized based on:

| Factor | Weight |
|--------|--------|
| Community interest | High |
| Development effort | Medium |
| Strategic alignment | High |
| Maintainability | Medium |
| Performance impact | Medium |

### Most Requested Features

Current top community requests:

1. Visual scripting
2. More platform support
3. VR integration
4. Advanced networking
5. Asset marketplace

## Deprecation Policy

### Pre-1.0

During alpha/beta:
- APIs may change with each minor version
- Breaking changes documented in changelog
- Migration guides provided for significant changes

### Post-1.0

After 1.0 release:
- Deprecated features marked with `#[deprecated]`
- 2 minor versions before removal
- Clear migration documentation

## Release Schedule

### Versioning

We follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes

### Release Cadence

| Phase | Cadence |
|-------|---------|
| Alpha | As needed |
| Beta | Monthly |
| Stable | Quarterly |

### Support Policy

| Version | Support Duration |
|---------|-----------------|
| Current | Full support |
| Previous minor | Security fixes |
| Older | Community only |

## Contributing to Roadmap

### How to Help

1. **Implement Features**: Check `help wanted` issues
2. **Provide Feedback**: Test pre-release versions
3. **Write Documentation**: Help document new features
4. **Create Examples**: Build showcase projects

### RFC Process

For major changes:

1. Create RFC in Discussions
2. Gather community feedback
3. Core team review
4. Implementation (if approved)

### Roadmap Updates

This roadmap is updated:
- After each major release
- Quarterly for minor adjustments
- As needed for significant changes

## Milestones

### Completed Milestones

| Milestone | Date | Highlights |
|-----------|------|------------|
| Initial Release | 2025 Q4 | Core ECS, basic rendering |
| AI Integration | 2025 Q4 | LLM support, behavior trees |

### Upcoming Milestones

| Milestone | Target | Goals |
|-----------|--------|-------|
| Beta Release | 2026 Q2 | Stable APIs, documentation |
| Editor Preview | 2026 Q3 | Basic visual editor |
| 1.0 Release | 2027 Q1 | Production ready |

## Risk Factors

### Technical Risks

| Risk | Mitigation |
|------|------------|
| LLM API changes | Abstraction layer, multiple providers |
| Performance challenges | Continuous benchmarking, optimization focus |
| Platform compatibility | Early testing on all platforms |

### Project Risks

| Risk | Mitigation |
|------|------------|
| Scope creep | Clear prioritization, phased releases |
| Maintainer burnout | Community building, shared ownership |
| Funding | Exploring sustainability options |

## Community Involvement

The roadmap reflects community input:

- **Surveys**: Periodic community surveys
- **Voting**: Feature request voting on GitHub
- **Discussions**: Open planning discussions
- **Contributors**: Active contributors help shape direction

## Related Documentation

- [Contributing](../dev/contributing.md) - How to contribute
- [Community](community.md) - Join the community
- [FAQ](faq.md) - Frequently asked questions
- [Best Practices](best-practices.md) - Current best practices
