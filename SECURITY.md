# Security Policy

## Supported Versions

Security fixes are applied on the `main` branch. The supported line is workspace version **0.4.0**, from `[workspace.package] version` in the root `Cargo.toml`.

| Version | Supported |
| --- | --- |
| `0.4.0` on `main` | Yes |
| `0.3.0`, `0.2.0`, and `0.1.0` | No |

`CHANGELOG.md` records 0.4.0, 0.3.0, 0.2.0, and 0.1.0. This repository has no `v0.4.0`, `v0.3.0`, `v0.2.0`, or `v0.1.0` tag. The older changelog entries are not supported releases.

The root `Cargo.toml` sets `[workspace.package] publish = false`. A member inherits that only when its own manifest sets `publish.workspace = true` (for example `astraweave-veil`; `astraweave-core` writes the same field as `publish = { workspace = true }`) or sets `publish = false` itself (for example `tools/xtask`). Members that omit `publish`, including `astraweave-sdk` and `astraweave-render`, do not inherit it. An omitted `publish` key leaves Cargo's default, which allows publishing.

Most crates still declare `version = "0.1.0"` in their own `Cargo.toml`. That number is not a separate supported release. Support follows current `main`, at workspace version 0.4.0. `astraweave-sdk` declares `0.4.0`. Crates with `version.workspace = true` inherit `0.4.0`.

The GitHub releases `assets-v1` (asset packs) and `aravath-greybox` (a prerelease greybox build of one commit) are not supported engine versions. Security fixes stay on `main`.

## Reporting a Vulnerability

Report a vulnerability in private through GitHub private vulnerability reporting:

<https://github.com/lazyxeon/AstraWeave/security/advisories/new>

Open the repository **Security** tab and choose **Report a vulnerability**. You need a GitHub account. The draft stays visible to you and the maintainers.

Include:

- A detailed description of the issue
- Steps to reproduce
- Potential impact assessment
- Any suggested mitigations
- The commit you tested, or workspace version `0.4.0` on `main`

We request a 72-hour window before public disclosure to coordinate fixes and releases.

## Responsible Disclosure

- Do not create a public GitHub issue for a security report.
- Avoid sharing proof-of-concept code publicly before we address the vulnerability.
- We will acknowledge receipt within 48 hours and provide status updates until resolution.

## Security Update Process

1. Confirm and reproduce the report in a secure environment.
2. Prioritize remediation based on severity using CVSS.
3. Develop patches and add regression tests when applicable.
4. For a vulnerability, coordinate release notes in `CHANGELOG.md`, and publish the GitHub security advisory when the fix is ready. A code-of-conduct report submitted through the form above is not a vulnerability advisory. Submitting that form attaches the reporter's GitHub account to the advisory as a collaborator, with a pending credit visible to maintainers. The report opens in the Triage state. Click **Close security advisory**. Do not click **Accept and open as draft**, and do not publish the advisory. Accepting it opens a draft that can still be published.
5. Notify the reporter once a vulnerability fix is released.

## Scope

This policy covers all crates under the AstraWeave workspace, tooling in `tools/`, and official Docker/installer artifacts.

Thank you for helping keep the AstraWeave community secure.
