# Security Audit Quick Reference

## Running Security Audits

### Quick Check
```bash
# Install cargo-audit if needed
cargo install cargo-audit --locked

# Run audit
cargo audit

# Expected result: "warning: 5 allowed warnings found" (no vulnerabilities)
```

### Detailed Analysis
```bash
# JSON output for parsing
cargo audit --json

# Deny warnings (for CI strict mode)
cargo audit --deny warnings

# Update advisory database
cargo audit --sync
```

## Current Security Status (as of 2025-10-12)

### ✅ Vulnerabilities: ZERO

**Last Scan**: 2025-10-12  
**Status**: All clear - no active security vulnerabilities

### ⚠️ Acknowledged Warnings: 5 (Unmaintained Crates)

These are **not security vulnerabilities**, just maintenance notices:

| Advisory | Crate | Status | Notes |
|----------|-------|--------|-------|
| RUSTSEC-2025-0052 | async-std | Discontinued | Used in example only |
| RUSTSEC-2023-0089 | atomic-polyfill | Unmaintained | Transitive (heapless → postcard) |
| RUSTSEC-2025-0057 | fxhash | No longer maintained | Transitive (sled → aw-net-server) |
| RUSTSEC-2024-0384 | instant | Unmaintained | Transitive (rhai) |
| RUSTSEC-2024-0436 | paste | No longer maintained | Transitive (tokenizers) |

## Recently Fixed (Historical Record)

### 2025-10-12: Critical Security Fixes

1. **RUSTSEC-2021-0070** - nalgebra 0.26.2 memory corruption
   - **Fix**: Removed unused mikktspace dependency
   - **Impact**: Zero (dependency was not used)

2. **RUSTSEC-2024-0437** - protobuf 2.28.0 DoS vulnerability
   - **Fix**: Updated prometheus 0.13 → 0.14
   - **Impact**: protobuf 2.28.0 → 3.7.2

See [SECURITY_VULNERABILITY_FIX_2025_10_12.md](root-archive/SECURITY_VULNERABILITY_FIX_2025_10_12.md) for full details.

## Interpreting OpenSSF Reports

OpenSSF Scorecard may report issues in these formats:

- **RUSTSEC-YYYY-NNNN**: RustSec advisory ID (primary reference)
- **GHSA-xxxx-xxxx-xxxx**: GitHub Security Advisory (may be duplicate)
- **CVE-YYYY-NNNNN**: Common Vulnerabilities and Exposures ID
- **PYSEC-YYYY-NNN**: Python advisory (not applicable to Rust)

**Note**: Count unique RUSTSEC IDs to avoid duplicates.

## Dependency Management

### Update Dependencies
```bash
# Update all dependencies
cargo update

# Update specific package
cargo update --package <crate_name>

# Update and check security
cargo update && cargo audit
```

### Check Dependency Tree
```bash
# Find which crate uses a dependency
cargo tree -i <crate_name>

# Check for duplicate versions
cargo tree --duplicates
```

## When to Act

### 🚨 IMMEDIATE ACTION REQUIRED
- Any vulnerability with severity **Critical** or **High**
- CVEs with active exploits
- Memory safety issues (corruption, use-after-free, etc.)
- Security advisories marked "vulnerability"

### ⚠️ PLAN MITIGATION
- Medium severity vulnerabilities
- Unmaintained crates with no known vulnerabilities
- Deprecated but still functional dependencies

### ℹ️ INFORMATIONAL ONLY
- Low severity issues with workarounds
- Unmaintained crates in examples/tools only
- Transitive dependencies with no direct exposure

## Adding to Ignore List

**Only for unmaintained warnings, never for vulnerabilities!**

Edit `deny.toml`:
```toml
[advisories]
ignore = [
    "RUSTSEC-YYYY-NNNN",  # brief description
]
```

## CI Integration

### GitHub Actions Workflow
Located in `.github/workflows/security-audit.yml`

Runs on:
- Pull requests (dependency changes)
- Weekly schedule
- Manual trigger

### Expected CI Behavior
- ✅ **Pass**: No vulnerabilities (warnings OK)
- ❌ **Fail**: Any active vulnerabilities found

## Troubleshooting

### "couldn't check if package is yanked" Errors
These are **warnings**, not failures. Caused by:
- Network timeouts to crates.io
- Rate limiting
- Temporary registry issues

**Solution**: Re-run audit or use `--db` flag for offline mode

### Stale Advisory Database
```bash
# Update database manually
cargo audit --sync

# Or delete and re-fetch
rm -rf ~/.cargo/advisory-db
cargo audit
```

### False Positives
If cargo audit reports a vulnerability that doesn't apply:
1. Verify the dependency tree (is it actually used?)
2. Check the advisory details (does it apply to your use case?)
3. If legitimate false positive, file issue with RustSec
4. **Never** ignore real vulnerabilities - fix or mitigate instead

## Resources

- **RustSec Database**: https://rustsec.org/
- **cargo-audit**: https://github.com/RustSec/rustsec/tree/main/cargo-audit
- **cargo-deny**: https://embarkstudios.github.io/cargo-deny/
- **OpenSSF Scorecard**: https://github.com/ossf/scorecard
- **AstraWeave Security Guide**: [SECURITY_AUDIT_GUIDE.md](SECURITY_AUDIT_GUIDE.md)

## Contact

Security concerns? See [SECURITY.md](../SECURITY.md) for reporting guidelines.

---

**Last Updated**: 2025-10-12  
**Status**: ✅ All security vulnerabilities resolved
