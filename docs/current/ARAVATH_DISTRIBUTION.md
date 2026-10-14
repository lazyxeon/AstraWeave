# Aravath Windows distribution

The Chrome "Dangerous download" warning on `aravath_greybox.exe` is a reputation warning. It is not evidence that the zip contains a known virus. This note says what the build does, what a certificate can change, and what it cannot.

## What the zip is

The Windows workflow (`.github/workflows/aravath-windows.yml`) builds `aravath_greybox.exe` with the `dist` profile (`strip = true`, thin LTO, the same unwind and overflow behavior as `release`). It stages the exe, `assets/aravath`, `README.txt`, `LICENSE`, and `SHA256SUMS.txt`. A separate job writes a CycloneDX SBOM, `aravath_greybox.cdx.json`.

On a push to `cursor/aravath-*`, the same workflow updates one rolling prerelease named `aravath-greybox`. A GitHub Release is a better download origin than an Actions artifact zip: the URL is a normal release asset on `github.com`, and browsers have seen that pattern before. It does not make a brand-new unsigned exe trusted.

The exe:

- embeds a `VERSIONINFO` (ProductName Aravath, CompanyName AstraWeave, FileDescription "Aravath greybox demo", version 0.4.0.0) and an `asInvoker` manifest (`uiAccess=false`)
- does not request administrator rights
- does not link an HTTP client (`reqwest`, `hyper`, `ureq`, `openssl`) and does not enable `tokio`'s `net` feature
- does not contain an updater
- reads `assets/aravath` and `aravath.cfg` only from the folder that contains the exe
- writes saves, `aravath.log`, and `aravath_crash.log` under `%LOCALAPPDATA%\Aravath`

`aravath.cfg` is a small UTF-8 text file. The only keys that do anything are `volumetrics=on` and `volumetrics=off`. The file must be a regular file of at most 16 KiB. A symlink is refused.

Save files (`.awsv`) are length-capped before allocation (32 MiB compressed, 64 MiB after LZ4), checksummed, and refused when the file or `index.json` is a symlink. Player ids used as directory names must be ASCII `[A-Za-z0-9_-]` and must not be Windows device names (`CON`, `COM1`, and the rest). The greybox edit log stores densities and the player record as eight hex digits of `f32::to_bits`. Decimal text from older saves still loads. The loader rejects non-finite numbers, including NaN and infinity written as those bits, coordinates outside ±2048, and more than 16384 edits.

## What still triggers the warning

Chrome Safe Browsing and Windows SmartScreen score a file by its signature and by how many people have already downloaded that exact file (or files from the same publisher). A new unsigned exe inside a zip, especially one that came from an Actions artifact, has no reputation. Both systems warn. That is expected.

These do **not** clear the warning by themselves:

- a version resource and manifest
- stripping symbols
- SHA-256 checksums
- a CycloneDX SBOM
- cargo-audit / cargo-deny being clean
- publishing the same unsigned bytes on a GitHub Release

A checksum lets a person confirm the zip was not swapped after it was built. Browsers do not read `SHA256SUMS.txt`.

## Code signing, and what it does not buy

A publicly trusted Authenticode signature gives SmartScreen a publisher to attach reputation to. Reputation still has to accumulate. Microsoft's current guidance (Windows app code-signing options, 2024 onward) is that an Extended Validation certificate no longer skips SmartScreen on first download. A freshly signed file from a new publisher can still show "Windows protected your PC" until enough people run it and don't report it.

Chrome's "Dangerous download" verdict is separate. Google does not publish a way to pre-register a new exe. Signing and hosting on a stable URL help over time. They do not guarantee the next download is clean.

Signing is not in the workflow yet. The published exe is unsigned, and the warnings above still apply. After you pick an option below, the matching step can be added. `scripts/sign-aravath.ps1` is the PFX helper and is not called by CI.

### Option A — Azure Artifact Signing (formerly Trusted Signing)

Public list price from Microsoft Learn (confirm in the Azure portal; it changes):

| SKU | Price | Included signatures / month |
| --- | --- | --- |
| Basic | $9.99 per account per month | 5,000, then $0.005 each |
| Premium | $99.99 per account per month | 100,000, then $0.005 each |

Constraints that matter:

- A **paid** Azure subscription. Free, trial, and sponsored subscriptions are rejected.
- The monthly fee is not pro-rated.
- Organizations: United States, Canada, the EU, and the UK.
- Individual developers: United States and Canada only.
- It does **not** issue an EV certificate. Microsoft's guidance is that this is still the right option for a non-Store exe when you are eligible.
- Identity validation takes time. A new certificate profile has no SmartScreen reputation on day one.

Secrets (all of them, or the step is skipped):

| Secret | Value |
| --- | --- |
| `AZURE_TENANT_ID` | Directory (tenant) id of the app registration |
| `AZURE_CLIENT_ID` | Application (client) id |
| `AZURE_CLIENT_SECRET` | Client secret **value** (not the secret id) |
| `AZURE_CODESIGN_ENDPOINT` | Regional endpoint, for example `https://eus.codesigning.azure.net/` |
| `AZURE_CODESIGN_ACCOUNT` | Artifact Signing account name |
| `AZURE_CODESIGN_PROFILE` | Certificate profile name |

The app registration needs the Artifact Signing Certificate Profile Signer role on that profile. When this option is chosen, add `azure/artifact-signing-action@v2` to the Windows job with those secrets. It is not in the workflow today.

### Option B — your own OV or EV certificate (PFX)

Microsoft's published ranges, as of their code-signing options page: an Organization Validation certificate is about $150–300 per year; an EV certificate is about $400 or more per year and usually needs a hardware token. Buying EV only to skip SmartScreen is no longer justified: since 2024 an EV signature goes through the same reputation check as OV.

Export the cert as a PFX (if the CA allows export; a token-backed EV key often cannot be exported, and then this path does not apply). Store it as secrets:

| Secret | Value |
| --- | --- |
| `WINDOWS_PFX_BASE64` | Base64 of the `.pfx` file, one line, no extra whitespace |
| `WINDOWS_PFX_PASSWORD` | Password for that PFX |

`scripts/sign-aravath.ps1` runs `signtool sign /fd SHA256 /td SHA256` with the DigiCert RFC 3161 timestamp server when `WINDOWS_PFX_BASE64` and `WINDOWS_PFX_PASSWORD` are set, and exits 0 when they are absent. CI does not call it until this option is chosen.

### Option C — SignPath Foundation (free for open source)

[SignPath Foundation](https://signpath.org/) signs qualifying open-source projects at no certificate cost. The project needs a public repo, an OSI license (this repo is MIT), and releases that are free to download. They hold the key. You do not put a PFX in GitHub secrets.

This workflow does not call SignPath. Their signing request is a GitHub App plus a workflow they approve, and a request submitted before approval fails. After they accept the project, add their submit-signing-request step on the Release job and point it at `aravath_greybox-windows.zip`. Their required attribution line is whatever the approval mail specifies (commonly a note that signing is provided by SignPath.io, certificate by SignPath Foundation).

## How to check a download yourself

In PowerShell, from the extracted folder:

```powershell
Get-FileHash -Algorithm SHA256 .\aravath_greybox.exe
```

Compare the hash to the `aravath_greybox.exe` line in `SHA256SUMS.txt` from the same zip. `aravath_greybox.cdx.json` is the CycloneDX bill of materials for the `aravath_runtime` crate graph.

## Dependency audit

`.github/workflows/aravath-windows.yml` runs `cargo deny check` (advisories, bans, licenses, sources) and `cargo audit` on every Aravath push and pull request. `scripts/check-aravath-network.sh` fails the job if the greybox graph grows an HTTP client or `tokio`/`net`.

What was actually removed or narrowed:

- `async-std` was only used by `examples/orchestrator_async_tick`. That example now uses `pollster`. Advisory `RUSTSEC-2025-0052` (async-std discontinued) no longer applies.
- `postcard` no longer enables its default `heapless` feature, which was pulling unmaintained `atomic-polyfill` (`RUSTSEC-2023-0089`) on non-Windows targets.
- `derivative` is not in the lockfile. Its old ignore entry was stale.
- The greybox does not enable `image`'s AVIF encoder (`ravif` / `rav1e`). PNG, JPEG, and the other formats the renderer already opens stay on.
- `astraweave-scene` no longer inherits the workspace `tokio` feature set (`full`, which includes `net`). `aw_asset_cli` declared `tokio` with `full` and never called it; that dependency is gone. `scripts/check-aravath-network.sh` is the regression check.
- Workspace `reqwest` and `aw-net-client`'s WebSocket client use rustls instead of native-tls, so `openssl` is not in the lockfile. `deny.toml` still bans it.

Patched in this pass (lockfile bumps, still inside the existing semver ranges unless noted): `bytes` 1.12.1, `crossbeam-epoch` 0.9.21, `h2` 0.4.20, `lz4_flex` 0.11.6, `quinn-proto` 0.11.19, `rustls` 0.23.45 with `rustls-webpki` 0.103.15, `tar` 0.4.46, `thin-vec` 0.2.21, `time` 0.3.55, `webbrowser` 1.2.4, `anyhow` 1.0.104, `event-listener` 5.4.2, `memmap2` 0.9.11, `rand` 0.8.8 and 0.9.5, `lru` 0.18.5, `steamworks` 0.13.1, `openh264` 0.8.1, `serial_test` 3.5 (drops vulnerable `scc` 2.4). `lz4_flex` is the one the greybox save loader actually calls.

What cannot be upgraded away. `deny.toml` and `.cargo/audit.toml` ignore these IDs, with the reason:

| Advisory | Crate | Why it remains |
| --- | --- | --- |
| `RUSTSEC-2024-0436` | `paste` 1.0.15 | Unmaintained proc-macro. No patched release. `nalgebra` / `simba` / `rapier3d` and `texture2ddecoder` require it, and the greybox links Rapier. |
| `RUSTSEC-2024-0384`, `RUSTSEC-2025-0057` | `instant`, `fxhash` | Unmaintained. Only through `sled` 0.34 in `aw-net-*`. Not in the greybox graph. |
| `RUSTSEC-2025-0141` | `bincode` 1.3 and 2.0 | Unmaintained, no patched release. Used for terrain, AI, and persistence-ecs blobs. Replacing it changes those formats. |
| `RUSTSEC-2025-0119`, `RUSTSEC-2025-0123`, `RUSTSEC-2025-0134`, `RUSTSEC-2026-0249`, `RUSTSEC-2026-0192`, `RUSTSEC-2024-0449` | `number_prefix`, `opentelemetry-jaeger`, `rustls-pemfile`, `smartstring`, `ttf-parser`, `unsafe-libyaml` | Unmaintained, no patched release. None of these are an HTTP client. `rustls-pemfile` loads certificates in `aw-net-server`. |
| `RUSTSEC-2021-0070` | `nalgebra` 0.26.2 | Deserialize can break a length invariant. The only crate that still pulls 0.26 is `mikktspace` 0.3.0, which is the latest release of that crate. The renderer uses it to generate tangents. It does not deserialize `nalgebra`. |
| `RUSTSEC-2026-0194`, `RUSTSEC-2026-0195` | `quick-xml` 0.36 and 0.37 | DoS in XML parsing. Reached through `wayland-scanner` (winit) and `zbus_xml` (accesskit). They parse protocol XML. No upgrade fits inside `winit` 0.30. |
| `RUSTSEC-2026-0049`, `RUSTSEC-2026-0098`, `RUSTSEC-2026-0099`, `RUSTSEC-2026-0104` | `rustls-webpki` 0.102.8 | Old webpki used by `rustls` 0.22 / `tokio-rustls` 0.25 in `aw-net-server`. Not linked into `aravath_greybox`. The patched line is 0.103.13+, which needs `tokio-rustls` 0.26 and a different `ServerCertVerifier` than the net TLS tests implement. |

Duplicate crate versions are reported (`multiple-versions = "warn"`) and do not fail the build. The duplicates are mostly `windows-sys` / `windows-targets` stacks pulled by different wgpu and winit versions, and collapsing them means upgrading wgpu. `spin` 0.10.0 is yanked and still required by `bevy_platform` 0.17.3; `cargo update -p bevy_platform` does not remove it. `cargo deny` warns on that yank and does not fail.

Those three are maintenance advisories, not a CVSS vulnerability with a patch. Replacing `sled` would be a storage-engine change in the net stack and is out of scope for this demo.

## VirusTotal and packers

This environment cannot link the MSVC `basis-universal` build, so no Windows exe was uploaded to VirusTotal from here.

The dist build does not use UPX or any other packer. Packers are a common source of false positives; leaving the exe unpacked avoids that class. Unsigned Rust exes are still often flagged by machine-learning detections with generic names (`Trojan.Generic`, `Wacatac`) because the model has little reputation data for that hash. A signature from one of the options above is what moves a file out of that bucket over time. A clean VirusTotal report on an unknown hash does not stop Chrome or SmartScreen.

## Decision

Pick one:

1. Add the Azure secrets, if you have a paid Azure subscription and you are in a supported region.
2. Add the PFX secrets, if you already have an exportable OV or EV certificate.
3. Apply to SignPath Foundation and, after approval, add their signing step.
4. Ship the unsigned Release anyway, and expect the warning until you do one of the above.

Until one of those is in place, the honest label on the download is: open-source demo, unsigned, no network, writes only under `%LOCALAPPDATA%\Aravath`.
