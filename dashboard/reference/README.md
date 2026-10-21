# Reference Documentation

Technical reference for AstraWeave engine internals.

---

## Engine Core

| Document | Description |
|----------|-------------|
| [Interfaces.md](Interfaces.md) | Core interface definitions |
| [engine-api.md](engine-api.md) | Engine API overview |
| [error_codes.md](error_codes.md) | Error code taxonomy |

---

## Standards & Conventions

| Document | Description |
|----------|-------------|
| [naming_and_style.md](naming_and_style.md) | Naming conventions and code style |
| [platform_matrix.md](platform_matrix.md) | Platform support matrix |
| [perf_budgets.md](perf_budgets.md) | Performance budgets |

---

## Schemas & Data

| Document | Description |
|----------|-------------|
| [authoring_schemas.md](authoring_schemas.md) | Asset and content schemas |
| [asset_ids_and_cache.md](asset_ids_and_cache.md) | Asset ID format and caching |

---

## API Documentation

For detailed API documentation, run:

```bash
cargo doc --open --no-deps
```

This generates HTML documentation for all public APIs across the workspace.

---

*See also: [Developer Guides](../guides/README.md)*
