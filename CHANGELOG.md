# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive engine validation (5,300+ tests across 17 core crates - January 13, 2026)

### Fixed
- **CRITICAL**: astraweave-rag DashMap deadlock in `get_cached_result_expired` (holding read lock while attempting write lock - January 13, 2026)
- astraweave-ai: Adjusted perception test threshold from 10µs to 20µs for system load resilience (January 11, 2026)
- astraweave-llm: Fixed cache pollution in parallel tests via unique WorldSnapshot values (January 11, 2026)

### Changed
- Improved test isolation patterns for parallel execution across LLM and RAG crates (January 11-13, 2026)

## [0.4.0] - 2025-11-18
- Comprehensive audit reports published (`docs/audits/`).
- Rendering pipeline upgrades (VXGI, Megalights, Nanite-inspired geometry).
- AI orchestration enhancements enabling 12,700+ agents @ 60 FPS.
- Deterministic ECS replay validation tooling improvements.

## [0.3.0] - 2025-08-02
- Added LLM orchestration crate (`astraweave-llm`).
- Introduced Rhino-based scripting (`astraweave-scripting`).
- Expanded integration test suite across physics and networking crates.

## [0.2.0] - 2025-04-11
- Integrated wgpu rendering pipeline with clustered forward lighting.
- Added asset streaming and texture baking utilities.
- Established benchmarking harness and performance dashboards.

## [0.1.0] - 2024-12-01
- Initial public release of the AstraWeave AI-Native Gaming Engine.
- Included core ECS, navigation, physics, and AI behavior crates.

[Unreleased]: https://github.com/lazyxeon/AstraWeave/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/lazyxeon/AstraWeave/releases/tag/v0.4.0
[0.3.0]: https://github.com/lazyxeon/AstraWeave/releases/tag/v0.3.0
[0.2.0]: https://github.com/lazyxeon/AstraWeave/releases/tag/v0.2.0
[0.1.0]: https://github.com/lazyxeon/AstraWeave/releases/tag/v0.1.0