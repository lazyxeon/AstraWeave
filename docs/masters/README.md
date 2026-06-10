# Master Reports

**AUTHORITATIVE DOCUMENTATION** - These are the three master reports that track the overall state of the AstraWeave project.

> [!IMPORTANT]
> These documents MUST be updated on ANY significant change per the Master Report Maintenance Protocol in `.github/copilot-instructions.md`.

---

## Documents

| Report | Description | Update Threshold |
|--------|-------------|------------------|
| **[MASTER_ROADMAP.md](MASTER_ROADMAP.md)** | Strategic roadmap and priorities (canonical here; stub at `docs/current/`) | Work >4 hours or phase completion |
| **[MASTER_BENCHMARK_REPORT.md](MASTER_BENCHMARK_REPORT.md)** | Performance benchmarks (canonical here; stub at `docs/current/`) | Changes ±10% or new benchmark |
| **[MASTER_COVERAGE_REPORT.md](../current/MASTER_COVERAGE_REPORT.md)** | Test coverage metrics (canonical at `docs/current/`; stub here) | Changes ±5% per-crate or ±2% overall |

---

## Maintenance Protocol

### When to Update

- **MASTER_ROADMAP**: Completing phases, changing priorities, discovering gaps
- **MASTER_BENCHMARK_REPORT**: Performance changes >10%, new benchmarks
- **MASTER_COVERAGE_REPORT**: Coverage changes ±5% per-crate or ±2% overall

### Update Process

1. Open the relevant master report
2. Update the appropriate section with new data
3. Increment the version number in the header
4. Add an entry to the "Revision History" table

### Verification

```powershell
# Check last update dates
Get-Item docs/masters/MASTER_*.md | Select-Object Name, LastWriteTime
```

---

## Hard Rules

- ✅ ALWAYS check if master reports need updating after completing work
- ✅ ALWAYS update all three reports if thresholds exceeded
- ✅ ALWAYS increment version and add revision history entry
- ❌ NEVER skip master report updates
- ❌ NEVER let master reports become stale (>1 month without review)

---

*See `.github/copilot-instructions.md` Section "Master Report Maintenance Protocol" for complete details.*
