# Security Audit Workflow Guide

This guide explains the security audit setup for the AstraWeave project and how to use the security tools.

## Overview

The project uses multiple security audit tools to ensure dependency security and license compliance:

- **cargo-deny**: Comprehensive dependency auditing (bans, licenses, advisories, sources)
- **cargo-audit**: RustSec vulnerability scanning  
- **OpenSSF Scorecard**: Security posture monitoring
- **Dependabot**: Automated dependency updates

## Security Workflows

### Automated Security Audits

1. **security-audit.yml**: Runs on dependency changes and weekly schedule
   - Tests for banned dependencies
   - Validates dependency sources  
   - Checks for security advisories
   - Verifies license compliance
   
2. **dependency-management.yml**: Daily dependency monitoring
   - Security audits with multiple tools
   - Dependency analysis and reporting
   - Cache management for CI performance

3. **scorecard.yml**: OpenSSF Scorecard analysis
   - Weekly security posture assessment
   - SARIF report generation for GitHub Security tab

### Configuration Files

- **deny.toml**: Main configuration for cargo-deny
- **dependabot.yml**: Automated dependency update configuration  
- **Cargo.toml**: Workspace license inheritance

## Running Security Audits Locally

### Quick Security Check
```bash
# Run all security checks
cargo deny check all

# Run individual checks
cargo deny check bans        # Check for banned/duplicate dependencies
cargo deny check sources     # Validate dependency sources  
cargo deny check advisories  # Check for security advisories
cargo deny check licenses    # Verify license compliance

# RustSec vulnerability scan
cargo audit
```

### Understanding Output

**Warnings vs Errors:**
- ⚠️ **Warnings**: Duplicate dependencies, unmaintained crates (not security issues)
- ❌ **Errors**: Banned dependencies, license violations, security vulnerabilities

**Common Warnings:**
- Duplicate crate versions: Normal in large workspaces
- Unmaintained crates: Dependencies that are no longer maintained but not vulnerable

## Security Configuration

### Allowed Licenses
The project allows these license types (see `deny.toml`):
- MIT, Apache-2.0, BSD-3-Clause, BSD-2-Clause
- CC0-1.0, ISC, Zlib, MPL-2.0
- BSL-1.0, NCSA, OFL-1.1
- Unicode-3.0, Unicode-DFS-2016
- CDLA-Permissive-2.0, LicenseRef-UFL-1.0

### Banned Dependencies
- OpenSSL crates (prefer rustls)
- Old rand_core versions (<0.6)

### Ignored Advisories
Current ignored advisories (not security vulnerabilities):
- RUSTSEC-2024-0388: derivative crate unmaintained
- RUSTSEC-2024-0384: instant crate unmaintained  
- RUSTSEC-2024-0436: paste crate unmaintained
- RUSTSEC-2025-0052: async-std has been discontinued
- RUSTSEC-2023-0089: atomic-polyfill is unmaintained
- RUSTSEC-2025-0057: fxhash no longer maintained

### Recently Resolved Vulnerabilities
- **RUSTSEC-2021-0070** (2025-10-12): nalgebra 0.26.2 memory corruption vulnerability
  - **Fix**: Removed unused mikktspace dependency from astraweave-render
  - **Impact**: No functional changes; mikktspace was not used in code
  
- **RUSTSEC-2024-0437** (2025-10-12): protobuf 2.28.0 DoS via uncontrolled recursion
  - **Fix**: Updated prometheus from 0.13 to 0.14 in astraweave-observability
  - **Impact**: Protobuf updated from 2.28.0 to 3.7.2 (secure version)

## Troubleshooting

### Network Timeouts
If cargo-audit fails with network timeouts:
```bash
# Use offline mode
cargo audit --db ~/.cargo/advisory-db

# Or update database manually  
cargo audit --sync
cargo audit
```

### License Issues
If license checks fail for workspace crates:
```bash
# Check workspace license inheritance
grep -r "license" */Cargo.toml

# All workspace crates should have:
# license.workspace = true
```

### Dependency Updates
To update dependencies and check security:
```bash
# Update Cargo.lock
cargo update

# Check for security issues
cargo deny check all
cargo audit
```

## Contributing Security Fixes

When adding new dependencies:
1. Ensure licenses are compatible (see allowed list in deny.toml)
2. Run security audits before submitting PR
3. Add any new required licenses to deny.toml if needed
4. Update ignore list only for maintenance warnings, not vulnerabilities

## Security Best Practices

1. **Keep dependencies updated**: Regular `cargo update` and Dependabot PRs
2. **Review security advisories**: Address RustSec vulnerabilities promptly  
3. **Use secure defaults**: Prefer rustls over OpenSSL, use latest crate versions
4. **Monitor security workflows**: Check GitHub Security tab for alerts
5. **License compliance**: Ensure all dependencies have compatible licenses

## Further Reading

- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-deny Documentation](https://embarkstudios.github.io/cargo-deny/)
- [OpenSSF Scorecard](https://github.com/ossf/scorecard)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)