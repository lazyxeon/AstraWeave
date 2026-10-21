# Naming & Style (MVP)

## Crate & Module Names
- Crates: `astraweave-<domain>` (e.g., `astraweave-render`, `astraweave-ai`)
- Modules: snake_case; types: UpperCamelCase; traits end with `Ext` only if extension traits

## File Layout
- `src/lib.rs` exposes minimal public API; `src/<module>.rs` or `src/<module>/mod.rs` for submodules
- `examples/` one crate per demo

## API Conventions
- Return `anyhow::Result<T>` at crate boundaries; use concrete error types internally
- Use `&mut self` for engine state mutation; prefer immutable data + explicit commands for systems
- Version fields on externally serialized types

## Docs & Tests
- Rustdoc on public items; examples compile via `doctest`
- Unit tests colocated; integration tests under `tests/`
